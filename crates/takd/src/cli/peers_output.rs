use std::path::PathBuf;

use anyhow::{Context, Result};
use tak_proto::local_daemon::v2::{
    Operation, RemoteStatusEntry, Request, Response, decode_response, encode_request,
};
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
        Response::RemoteStatus { remotes, .. } => {
            let peers = remotes.into_iter().map(PeerRow::from).collect::<Vec<_>>();
            print!("{}", render_peers(&peers));
            Ok(())
        }
        Response::Error { code, .. } => anyhow::bail!(
            "local daemon rejected protocol v2 peers request ({code:?}); upgrade tak and takd together"
        ),
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
    let request = Request {
        request_id: "peers".to_string(),
        operation: Operation::GetRemoteStatus {
            node_ids: Vec::new(),
        },
    };
    let encoded = encode_request(&request)?;
    stream.write_all(encoded.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.shutdown().await?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).await?;
    decode_response(line.trim_end().as_bytes(), &request.request_id).map_err(Into::into)
}

fn render_peers(peers: &[PeerRow]) -> String {
    let mut output =
        String::from("NODE         TRANSPORT  STATE        LAST_HEARTBEAT  JOBS  QUEUE\n");
    if peers.is_empty() {
        output.push_str("no remote workers configured\n");
        return output;
    }
    for peer in peers {
        output.push_str(&format!(
            "{:<12} {:<10} {:<12} {:<15} {:<5} {}\n",
            peer.node_id,
            peer.transport,
            peer.state,
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

struct PeerRow {
    node_id: String,
    transport: String,
    state: String,
    last_heartbeat_ms: Option<i64>,
    active_job_count: Option<u32>,
    queue_depth: Option<u32>,
}

impl From<RemoteStatusEntry> for PeerRow {
    fn from(status: RemoteStatusEntry) -> Self {
        if let Some(peer) = status.peer {
            return Self {
                node_id: peer.node_id,
                transport: peer.transport,
                state: peer.state,
                last_heartbeat_ms: peer.last_heartbeat_ms,
                active_job_count: peer.active_job_count,
                queue_depth: peer.queue_depth,
            };
        }
        let state = status
            .snapshot
            .as_ref()
            .map(|snapshot| {
                if snapshot.healthy {
                    "ready"
                } else {
                    "unhealthy"
                }
            })
            .unwrap_or("unavailable");
        Self {
            node_id: status.remote.node_id,
            transport: status.remote.transport,
            state: state.to_string(),
            last_heartbeat_ms: status
                .snapshot
                .as_ref()
                .and_then(|snapshot| i64::try_from(snapshot.sampled_at_ms).ok()),
            active_job_count: status
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.usage.execution_slots),
            queue_depth: status.snapshot.map(|snapshot| snapshot.queue_depth),
        }
    }
}

#[cfg(test)]
mod tests;
