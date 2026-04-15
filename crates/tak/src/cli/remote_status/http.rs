use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use prost::Message;
use tak_exec::{
    endpoint_host_port as shared_endpoint_host_port,
    endpoint_socket_addr as shared_endpoint_socket_addr,
};
use tak_proto::NodeStatusResponse;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::cli::remote_probe_support::{AbortOnDrop, ProbeAttemptError};

pub(super) async fn fetch_status_once<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeStatusResponse, ProbeAttemptError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (status, body) = send_request(stream, authority, bearer_token, base_url).await?;
    if status != 200 {
        return Err(ProbeAttemptError::final_error(anyhow!(
            "node status failed with HTTP {status}"
        )));
    }
    NodeStatusResponse::decode(body.as_slice())
        .context("decode node status protobuf")
        .map_err(ProbeAttemptError::final_error)
}

fn endpoint_socket_addr_inner(endpoint: &str) -> Result<String> {
    shared_endpoint_socket_addr(endpoint)
}

fn endpoint_host_port_inner(endpoint: &str) -> Result<(String, u16)> {
    shared_endpoint_host_port(endpoint)
}

pub(super) fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    endpoint_socket_addr_inner(endpoint)
}

pub(super) fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    endpoint_host_port_inner(endpoint)
}

async fn send_request<S>(
    stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<(u16, Vec<u8>), ProbeAttemptError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
            .await
            .context("read node status")
            .map_err(ProbeAttemptError::retryable)?;
    let _connection_task = AbortOnDrop::new(tokio::spawn(async move {
        let _ = connection.await;
    }));
    let request = Request::builder()
        .method("GET")
        .uri("/v1/node/status")
        .header(hyper::header::HOST, authority)
        .header(
            hyper::header::AUTHORIZATION,
            format!("Bearer {}", bearer_token.trim()),
        )
        .header(hyper::header::CONNECTION, "close")
        .body(Empty::<Bytes>::new())
        .context("write node status")
        .map_err(ProbeAttemptError::retryable)?;
    let response = sender
        .send_request(request)
        .await
        .context("read node status")
        .map_err(ProbeAttemptError::retryable)?;
    let status = response.status().as_u16();
    let body = response
        .into_body()
        .collect()
        .await
        .with_context(|| format!("truncated HTTP response body from {base_url}"))
        .map_err(ProbeAttemptError::retryable)?
        .to_bytes()
        .to_vec();
    Ok((status, body))
}

#[path = "http_tests.rs"]
mod http_tests;
