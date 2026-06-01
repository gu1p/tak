use super::*;

mod http2;
mod legacy_http;
mod request;
mod response;
mod target;
mod tor_client;

use http2::{BrokerHttp2Request, BrokerHttp2Response, BrokerHttp2StreamRequest};
use legacy_http::read_remote_http_response;
use request::{LocalBrokerRequest, LocalBrokerRequestHead, parse_broker_request};
use response::{BrokerHttpError, write_broker_error};
use target::{prefers_http2, prefers_http2_head, validate_target, validate_target_head};
use tor_client::BrokerBody;
pub(in crate::daemon::protocol) use tor_client::BrokerRemoteHttpRequest;
pub use tor_client::{BrokerForwardResponse, TorBroker};

use futures::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use tokio_util::io::ReaderStream;

const BROKER_VERSION_HEADER: &str = "X-Tak-Broker-Version";
const REMOTE_NODE_HEADER: &str = "X-Tak-Remote-Node";
const REMOTE_ENDPOINT_HEADER: &str = "X-Tak-Remote-Endpoint";
const REMOTE_PROTOCOL_HEADER: &str = "X-Tak-Remote-Protocol";
const REMOTE_TRANSPORT_HEADER: &str = "X-Tak-Remote-Transport";
const MAX_RESPONSE_HEADER_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 512 * 1024 * 1024;

pub(super) fn is_http_request_line(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let (Some(method), Some(_path), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    parts.next().is_none() && is_http_method(method) && version.starts_with("HTTP/")
}

fn is_http_method(method: &str) -> bool {
    matches!(
        method,
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    )
}

pub(super) async fn handle_broker_http_request<R, W>(
    broker: &TorBroker,
    peers: &crate::daemon::peer_manager::PeerManager,
    first_line: String,
    mut reader: R,
    writer: &mut W,
) -> Result<()>
where
    R: AsyncBufRead + Unpin + Send + Sync + 'static,
    W: AsyncWrite + Unpin,
{
    if first_line_has_streaming_upload_path(&first_line) {
        match request::parse_broker_request_head(first_line, &mut reader).await {
            Ok(request) => match forward_request_streaming(broker, peers, request, reader).await {
                Ok(response) => {
                    writer.write_all(&response).await?;
                    writer.flush().await?;
                    Ok(())
                }
                Err(err) => write_broker_error(writer, err).await,
            },
            Err(err) => write_broker_error(writer, err).await,
        }
    } else {
        match parse_broker_request(first_line, &mut reader).await {
            Ok(request) => match forward_request(broker, request).await {
                Ok(response) => {
                    writer.write_all(&response).await?;
                    writer.flush().await?;
                    Ok(())
                }
                Err(err) => write_broker_error(writer, err).await,
            },
            Err(err) => write_broker_error(writer, err).await,
        }
    }
}

fn first_line_has_streaming_upload_path(first_line: &str) -> bool {
    let mut parts = first_line.split_whitespace();
    matches!(parts.next(), Some("POST"))
        && parts.next().is_some_and(|path| {
            path.starts_with("/v2/workspaces/uploads/") && path.contains("/stream")
        })
}

async fn forward_request_streaming<R>(
    broker: &TorBroker,
    peers: &crate::daemon::peer_manager::PeerManager,
    request: LocalBrokerRequestHead,
    reader: R,
) -> std::result::Result<Vec<u8>, BrokerHttpError>
where
    R: AsyncBufRead + Unpin + Send + Sync + 'static,
{
    let target = validate_target_head(&request)?;
    if !prefers_http2_head(&request) {
        return Err(BrokerHttpError::bad_request("stream_upload_requires_http2"));
    }
    let mut headers = request.headers().to_vec();
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(hyper::header::AUTHORIZATION.as_str()))
        && let Some(peer_target) = peers.connection_target(&target.node_id)
    {
        headers.push((
            hyper::header::AUTHORIZATION.as_str().to_string(),
            format!("Bearer {}", peer_target.bearer_token),
        ));
    }
    let stream =
        ReaderStream::new(reader.take(request.content_length() as u64)).map_ok(Frame::data);
    let body: BrokerBody = StreamBody::new(stream).boxed();
    tracing::info!(
        node_id = %target.node_id,
        endpoint = %target.endpoint,
        path = request.path(),
        body_bytes = request.content_length(),
        "forwarding workspace upload stream over Tor"
    );
    let request = BrokerHttp2StreamRequest::from_parts(
        request.method(),
        request.path(),
        headers,
        body,
        &target.endpoint,
        &target.node_id,
    )?;
    broker
        .http2_exchange_stream(&target.endpoint, request)
        .await?
        .to_http1_bytes()
}

async fn forward_request(
    broker: &TorBroker,
    request: LocalBrokerRequest,
) -> std::result::Result<Vec<u8>, BrokerHttpError> {
    let target = validate_target(&request)?;
    if prefers_http2(&request) {
        let http2_request = BrokerHttp2Request::new(&request, &target.endpoint)?;
        match broker.http2_exchange(&target.endpoint, http2_request).await {
            Ok(response) => {
                return response.to_http1_bytes();
            }
            Err(err) if !can_fallback_after_http2_failure(request.method(), &err) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }
    let mut remote = broker
        .connect(&target.endpoint)
        .await
        .map_err(|err| BrokerHttpError::bad_gateway("connect_failed", err))?;
    let payload = request
        .forward_bytes(&target.endpoint)
        .map_err(|err| BrokerHttpError::bad_request_with_source("invalid_request", err))?;
    remote
        .write_all(&payload)
        .await
        .map_err(|err| BrokerHttpError::bad_gateway("write_failed", err))?;
    read_remote_http_response(&mut remote).await
}

fn can_fallback_after_http2_failure(method: &str, err: &BrokerHttpError) -> bool {
    err.code() == "http2_unavailable"
        || matches!(method, "GET" | "HEAD" | "OPTIONS" | "PUT" | "DELETE")
            && matches!(err.code(), "connect_failed" | "http2_request_failed")
}
