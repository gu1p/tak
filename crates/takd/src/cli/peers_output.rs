use std::path::PathBuf;

use anyhow::{Context, Result};
use takd::{PeerSnapshot, PeersListRequest, Request, Response};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

pub(super) async fn print_peers() -> Result<()> {
    let socket_path = daemon_socket_path();
    let response = send_peers_list(&socket_path).await.with_context(|| {
        format!(
            "takd serve is not reachable at daemon socket {}",
            socket_path.display()
        )
    })?;
    match response {
        Response::PeersSnapshot { peers, .. } => {
            print!("{}", render_peers(&peers));
            Ok(())
        }
        Response::Error { message, .. } => anyhow::bail!("{message}"),
        other => anyhow::bail!("unexpected daemon response: {other:?}"),
    }
}

fn daemon_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(takd::default_socket_path)
}

async fn send_peers_list(socket_path: &std::path::Path) -> Result<Response> {
    let mut stream = UnixStream::connect(socket_path).await?;
    let request = Request::PeersList(PeersListRequest {
        request_id: "peers".to_string(),
    });
    let encoded = serde_json::to_string(&request)?;
    stream.write_all(encoded.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.shutdown().await?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).await?;
    Ok(serde_json::from_str(line.trim_end())?)
}

fn render_peers(peers: &[PeerSnapshot]) -> String {
    let mut output =
        String::from("NODE         TRANSPORT  STATE        LAST_HEARTBEAT  JOBS  QUEUE\n");
    if peers.is_empty() {
        output.push_str("no tor peers configured\n");
        return output;
    }
    for peer in peers {
        output.push_str(&format!(
            "{:<12} {:<10} {:<12} {:<15} {:<5} {}\n",
            peer.node_id,
            peer.transport,
            peer.state.as_str(),
            peer.last_heartbeat_ms
                .map(|_| "seen".to_string())
                .unwrap_or_else(|| "never".to_string()),
            peer.active_job_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
            peer.queue_depth
                .map(|value| value.to_string())
                .unwrap_or_else(|| "?".to_string()),
        ));
    }
    output
}

#[cfg(test)]
mod tests;
