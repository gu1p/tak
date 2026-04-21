use anyhow::{Result, bail};
use futures::future::join_all;
use tak_core::model::RemoteTransportKind;
use tak_exec::{endpoint_host_port, endpoint_socket_addr, write_remote_observation};
use tak_proto::NodeStatusResponse;

use crate::cli::remote_probe_support::ProbeAttemptError;
use crate::cli::remote_probe_support::transport::{
    TorOnionRetryTexts, connect_remote, retry_tor_onion,
};

use super::http::fetch_status_once;
use super::{RemoteRecord, RemoteStatusResult};

pub(super) async fn fetch_snapshot(remotes: &[RemoteRecord]) -> Vec<RemoteStatusResult> {
    let mut results = join_all(remotes.iter().map(fetch_remote_status_result)).await;
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

async fn fetch_remote_status_result(remote: &RemoteRecord) -> RemoteStatusResult {
    let remote = remote.clone();
    match fetch_node_status(&remote.base_url, &remote.transport, &remote.bearer_token).await {
        Ok(status) => {
            if let Some(node) = status.node.as_ref() {
                let _ = write_remote_observation(node, status.sampled_at_ms);
            }
            RemoteStatusResult {
                remote,
                status: Some(status),
                error: None,
            }
        }
        Err(err) => RemoteStatusResult {
            remote,
            status: None,
            error: Some(err.to_string()),
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

    retry_tor_onion(
        base_url,
        &host,
        port,
        TorOnionRetryTexts {
            build_config: "build tor node status client config",
            bootstrap: "bootstrap tor node status client",
            connect: "connect node status",
            timeout_tail: " while requesting node status",
            no_retryable_error: "node status failed without a retryable error",
        },
        |stream| fetch_status_once(stream, &authority, bearer_token, base_url),
    )
    .await
}
