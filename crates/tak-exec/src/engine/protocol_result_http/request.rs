use std::time::Duration;

use anyhow::Context;

use crate::remote_endpoint::remote_protocol_bearer_token;

use super::super::{RemoteHttpExchangeError, StrictRemoteTarget, transport};

#[path = "request/connection_task.rs"]
mod connection_task;
#[path = "request/daemon.rs"]
mod daemon;
#[path = "request/response.rs"]
mod response;

use connection_task::AbortOnDrop;
use response::{malformed_response, read_response_with_headers, timeout_error};

#[derive(Debug, Clone)]
pub(crate) struct RemoteHttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: Vec<ResponseHeader>,
    pub(crate) body: Vec<u8>,
    pub(crate) daemon_task_handle: Option<String>,
    pub(crate) daemon_peer_node_id: Option<String>,
    pub(crate) daemon_peer_endpoint: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResponseHeader {
    pub(crate) name: String,
    pub(crate) value: String,
}

impl RemoteHttpResponse {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
    }
}

/// Sends a small HTTP request to a remote endpoint and returns `(status_code, body)`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn remote_protocol_http_request(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    phase: &str,
    timeout: Duration,
) -> std::result::Result<(u16, Vec<u8>), RemoteHttpExchangeError> {
    remote_protocol_http_request_with_extra_headers(target, method, path, body, phase, timeout, &[])
        .await
        .map(|response| (response.status, response.body))
}

pub(crate) async fn remote_protocol_http_request_with_extra_headers(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    phase: &str,
    timeout: Duration,
    extra_headers: &[(&str, String)],
) -> std::result::Result<RemoteHttpResponse, RemoteHttpExchangeError> {
    if transport::uses_tor_broker(target)
        .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?
    {
        return daemon::request_via_daemon(
            target,
            method,
            path,
            body,
            phase,
            transport::phase_timeout(target, timeout),
            extra_headers,
        )
        .await;
    }

    let socket_addr = remote_socket_addr(target)?;
    let bearer_token =
        remote_protocol_bearer_token(&target.node_id, &target.bearer_token, target.transport_kind)
            .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?;
    let payload = body.unwrap_or(&[]);

    let exchange = async {
        let stream = transport::connect(target)
            .await
            .map_err(|err| RemoteHttpExchangeError::connect(format!("{err:#}")))?;
        let (mut sender, connection) = handshake(stream.stream, target, phase).await?;
        let _connection_task = AbortOnDrop::new(tokio::spawn(async move {
            let _ = connection.await;
        }));
        let request = build_request(RequestInput {
            method,
            path,
            socket_addr: &socket_addr,
            bearer_token,
            payload,
            extra_headers,
        })?;
        let response = sender.send_request(request).await.map_err(|_| {
            RemoteHttpExchangeError::other(format!(
                "infra error: remote node {} returned malformed HTTP response for {}",
                target.node_id, phase
            ))
        })?;
        read_response_with_headers(target, phase, response, false).await
    };

    let effective_timeout = transport::phase_timeout(target, timeout);
    tokio::time::timeout(effective_timeout, exchange)
        .await
        .map_err(|_| timeout_error(target, phase))?
}

fn remote_socket_addr(target: &StrictRemoteTarget) -> Result<String, RemoteHttpExchangeError> {
    transport::socket_addr(target)
        .with_context(|| {
            format!(
                "infra error: remote node {} has invalid endpoint {}",
                target.node_id, target.endpoint
            )
        })
        .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))
}

async fn handshake(
    stream: transport::RemoteIoStream,
    target: &StrictRemoteTarget,
    phase: &str,
) -> Result<
    (
        hyper::client::conn::http1::SendRequest<http_body_util::Full<bytes::Bytes>>,
        hyper::client::conn::http1::Connection<
            hyper_util::rt::TokioIo<transport::RemoteIoStream>,
            http_body_util::Full<bytes::Bytes>,
        >,
    ),
    RemoteHttpExchangeError,
> {
    hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
        .await
        .map_err(|_| malformed_response(target, phase))
}

struct RequestInput<'a> {
    method: &'a str,
    path: &'a str,
    socket_addr: &'a str,
    bearer_token: Option<&'a str>,
    payload: &'a [u8],
    extra_headers: &'a [(&'a str, String)],
}

fn build_request(
    input: RequestInput<'_>,
) -> Result<hyper::Request<http_body_util::Full<bytes::Bytes>>, RemoteHttpExchangeError> {
    let mut request = hyper::Request::builder()
        .method(input.method)
        .uri(input.path)
        .header(hyper::header::HOST, input.socket_addr)
        .header(hyper::header::CONNECTION, "close")
        .header("X-Tak-Protocol-Version", "v1")
        .header(hyper::header::CONTENT_TYPE, "application/x-protobuf");
    if let Some(bearer_token) = input.bearer_token {
        request = request.header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {bearer_token}"),
        );
    }
    for (name, value) in input.extra_headers {
        request = request.header(*name, value);
    }
    request
        .body(http_body_util::Full::new(bytes::Bytes::copy_from_slice(
            input.payload,
        )))
        .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))
}
