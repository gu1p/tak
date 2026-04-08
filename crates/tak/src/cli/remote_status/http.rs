use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tak_exec::{
    endpoint_host_port as shared_endpoint_host_port,
    endpoint_socket_addr as shared_endpoint_socket_addr,
};
use tak_proto::NodeStatusResponse;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::cli::remote_probe_support::ProbeAttemptError;

pub(super) async fn fetch_status_once<S>(
    mut stream: S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> std::result::Result<NodeStatusResponse, ProbeAttemptError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let request = format!(
        "GET /v1/node/status HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write node status")
        .map_err(ProbeAttemptError::retryable)?;
    stream
        .flush()
        .await
        .context("flush node status")
        .map_err(ProbeAttemptError::retryable)?;
    let (status, body) = read_http_response(&mut stream, base_url)
        .await
        .map_err(ProbeAttemptError::retryable)?;
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

async fn read_http_response<S>(stream: &mut S, base_url: &str) -> Result<(u16, Vec<u8>)>
where
    S: AsyncRead + Unpin + ?Sized,
{
    let mut response = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let read = stream.read(&mut chunk).await.context("read node status")?;
        if read == 0 {
            bail!("malformed HTTP response from {base_url}");
        }
        response.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&response[..header_end]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("invalid HTTP status from {base_url}"))?;
    let content_length = http_content_length(&head, base_url)?;
    let mut body = response[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read node status body")?;
        if read == 0 {
            bail!("truncated HTTP response body from {base_url}");
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Ok((status, body))
}

fn http_content_length(head: &str, base_url: &str) -> Result<usize> {
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .with_context(|| format!("invalid HTTP content-length from {base_url}"));
        }
    }
    Ok(0)
}
