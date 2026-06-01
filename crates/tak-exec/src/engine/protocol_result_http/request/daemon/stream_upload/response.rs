use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::net::UnixStream;

use super::super::types::DaemonPeerSnapshot;
use crate::engine::protocol_result_http::request::{RemoteHttpResponse, ResponseHeader};

pub(super) async fn read_raw_http_response(
    peer: &DaemonPeerSnapshot,
    stream: UnixStream,
) -> Result<RemoteHttpResponse> {
    let mut reader = BufReader::new(stream);
    let status = read_status(&mut reader).await?;
    let (headers, content_length) = read_headers(&mut reader).await?;
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(RemoteHttpResponse {
        status,
        headers,
        body,
        daemon_task_handle: None,
        daemon_peer_node_id: Some(peer.node_id.clone()),
        daemon_peer_endpoint: Some(peer.endpoint.clone()),
    })
}

async fn read_status(reader: &mut BufReader<UnixStream>) -> Result<u16> {
    let mut status_line = String::new();
    if reader.read_line(&mut status_line).await? == 0 {
        bail!("local takd closed stream before upload response");
    }
    status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .context("parse upload response status")
}

async fn read_headers(reader: &mut BufReader<UnixStream>) -> Result<(Vec<ResponseHeader>, usize)> {
    let mut headers = Vec::new();
    let mut content_length = 0_usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            bail!("local takd closed stream before upload response headers");
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_string();
            let value = value.trim().to_string();
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.parse().unwrap_or(0);
            }
            headers.push(ResponseHeader { name, value });
        }
    }
    Ok((headers, content_length))
}
