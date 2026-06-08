use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tak_proto::{ErrorResponse, NodeStatusResponse};

pub(super) fn fetch_live_status(
    state_root: &Path,
    bearer_token: &str,
) -> Result<NodeStatusResponse> {
    let socket_path = state_root.join("agent-control.sock");
    let mut stream = UnixStream::connect(&socket_path).with_context(|| {
        format!(
            "takd serve is not reachable at control socket {}",
            socket_path.display()
        )
    })?;
    write!(
        stream,
        "GET /v1/node/status HTTP/1.1\r\nHost: takd-control\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n",
        bearer_token.trim()
    )?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let (status_code, body) = split_http_response(&response)?;
    if status_code != 200 {
        bail!(
            "takd live task status failed with HTTP {status_code}: {}",
            error_message(body)
        );
    }
    NodeStatusResponse::decode(body).context("decode live takd task status")
}

fn split_http_response(response: &[u8]) -> Result<(u16, &[u8])> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or_else(|| anyhow!("takd control socket returned malformed HTTP"))?;
    let head = std::str::from_utf8(&response[..split])
        .context("takd control socket returned non-utf8 HTTP headers")?;
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("takd control socket returned malformed status line"))?;
    Ok((status_code, &response[split..]))
}

fn error_message(body: &[u8]) -> String {
    ErrorResponse::decode(body)
        .map(|value| value.message)
        .unwrap_or_else(|_| "unknown_error".to_string())
}
