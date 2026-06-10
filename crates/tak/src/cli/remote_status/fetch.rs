use anyhow::{Result, bail};
use futures::future::join_all;
use tak_core::model::RemoteTransportKind;
use tak_exec::{endpoint_host_port, endpoint_socket_addr, write_remote_observation};
use tak_proto::NodeStatusResponse;

use crate::cli::remote_probe_support::ProbeAttemptError;
use crate::cli::remote_probe_support::transport::connect_remote;

use super::http::fetch_status_once;
use super::{RemoteRecord, RemoteStatusResult};

pub(in crate::cli) async fn fetch_snapshot(remotes: &[RemoteRecord]) -> Vec<RemoteStatusResult> {
    let mut results = join_all(remotes.iter().cloned().map(fetch_remote_status_result)).await;
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

pub(super) async fn fetch_remote_status_result(remote: RemoteRecord) -> RemoteStatusResult {
    match fetch_node_status(&remote.base_url, &remote.transport, &remote.bearer_token).await {
        Ok(status) => {
            if let Some(node) = status.node.as_ref() {
                let _ = write_remote_observation(node, status.sampled_at_ms);
            }
            RemoteStatusResult {
                remote,
                status: Some(status),
                error: None,
                peer: None,
            }
        }
        Err(err) => RemoteStatusResult {
            remote,
            status: None,
            error: Some(err.to_string()),
            peer: None,
        },
    }
}

async fn fetch_node_status(
    base_url: &str,
    transport: &str,
    bearer_token: &str,
) -> Result<NodeStatusResponse> {
    let kind = match transport {
        "direct" => RemoteTransportKind::Direct,
        "tor" => RemoteTransportKind::Tor,
        _ => bail!("unsupported remote transport `{transport}`"),
    };
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    if kind != RemoteTransportKind::Tor || !host.ends_with(".onion") {
        let stream = connect_remote(&host, port, kind).await?;
        return fetch_status_once(stream, &authority, bearer_token, base_url)
            .await
            .map_err(ProbeAttemptError::into_anyhow);
    }

    // Reaching here means a Tor `.onion` node was probed directly, which only
    // happens once the local takd peer query has already come back unavailable
    // (the daemon path owns Tor status and filters these out otherwise). Report
    // the real reason instead of the old message that wrongly implied takd was
    // not serving even when it was.
    bail!("{}", super::daemon::tor_status_daemon_unreachable_message())
}
