use super::*;

mod http2;
mod legacy_http;
mod request;
mod response;
mod target;
mod tor_client;

use http2::{BrokerHttp2Request, BrokerHttp2Response};
use legacy_http::read_remote_http_response;
use request::{LocalBrokerRequest, parse_broker_request};
use response::{BrokerHttpError, write_broker_error};
use target::{prefers_http2, validate_target};
pub(in crate::daemon::protocol) use tor_client::BrokerRemoteHttpRequest;
pub use tor_client::{BrokerForwardResponse, TorBroker};

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
    first_line: String,
    reader: &mut R,
    writer: &mut W,
) -> Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    match parse_broker_request(first_line, reader).await {
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

async fn forward_request(
    broker: &TorBroker,
    request: LocalBrokerRequest,
) -> std::result::Result<Vec<u8>, BrokerHttpError> {
    let target = validate_target(&request)?;
    if prefers_http2(&request) {
        let http2_request = BrokerHttp2Request::new(&request, &target.endpoint)?;
        match broker.http2_exchange(&target.endpoint, http2_request).await {
            Ok(response) => return response.to_http1_bytes(),
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
