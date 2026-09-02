use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures::future::join_all;
use prost::Message;
use tak_core::remote_inventory::RemoteRecord;
use tak_proto::NodeStatusResponse;
use tak_proto::local_daemon::v2::RemoteStatusEntry;
use tak_proto::worker_v2::WorkerSnapshot;

use super::{RemoteAccess, public_remote};

const UPGRADE: &str = "upgrade tak, takd, and workers together";

pub(super) async fn snapshot(
    access: &RemoteAccess,
    node_ids: &[String],
) -> Result<Vec<RemoteStatusEntry>> {
    let wanted = node_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let remotes = access
        .configured()?
        .remotes
        .into_iter()
        .filter(|remote| remote.enabled)
        .filter(|remote| wanted.is_empty() || wanted.contains(remote.node_id.as_str()));
    let mut results = join_all(remotes.map(|remote| direct_status(access, remote))).await;
    results.sort_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    Ok(results)
}

pub(super) async fn read(
    access: &RemoteAccess,
    node_id: &str,
    path: &str,
) -> Result<(u16, Vec<u8>)> {
    let remote = configured_remote(access, node_id)?;
    let response = access
        .broker
        .worker_v2_http_exchange(&target(&remote), "GET", path, &[])
        .await?;
    if response.status != 200 {
        return Ok((response.status, response.body));
    }
    let payload = tak_proto::worker_v2::decode_display_payload(&response.body)
        .map_err(|_| anyhow::anyhow!(UPGRADE))?;
    Ok((response.status, payload))
}

async fn direct_status(access: &RemoteAccess, remote: RemoteRecord) -> RemoteStatusEntry {
    match fetch_status(access, &remote).await {
        Ok((snapshot, detail)) => RemoteStatusEntry {
            remote: public_remote(remote),
            snapshot: Some(snapshot),
            detail_base64: Some(STANDARD.encode(detail)),
            error: None,
            peer: None,
        },
        Err(error) => RemoteStatusEntry {
            remote: public_remote(remote),
            snapshot: None,
            detail_base64: None,
            error: Some(error.to_string()),
            peer: None,
        },
    }
}

async fn fetch_status(
    access: &RemoteAccess,
    remote: &RemoteRecord,
) -> Result<(WorkerSnapshot, Vec<u8>)> {
    let target = target(remote);
    let snapshot_response = access
        .broker
        .worker_v2_http_exchange(&target, "GET", "/v2/worker/snapshot", &[])
        .await
        .context("fetch worker v2 snapshot")?;
    if snapshot_response.status != 200 {
        return remote_status_failure(snapshot_response.status);
    }
    let snapshot = tak_proto::worker_v2::decode_snapshot(&snapshot_response.body)
        .map_err(|_| anyhow::anyhow!(UPGRADE))?;
    if snapshot.node_id != remote.node_id {
        bail!(UPGRADE);
    }
    let detail_response = access
        .broker
        .worker_v2_http_exchange(&target, "GET", "/v2/worker/status", &[])
        .await
        .context("fetch worker v2 status detail")?;
    if detail_response.status != 200 {
        return remote_status_failure(detail_response.status);
    }
    let detail = tak_proto::worker_v2::decode_display_payload(&detail_response.body)
        .map_err(|_| anyhow::anyhow!(UPGRADE))?;
    let decoded =
        NodeStatusResponse::decode(detail.as_slice()).map_err(|_| anyhow::anyhow!(UPGRADE))?;
    if decoded.node.as_ref().map(|node| node.node_id.as_str()) != Some(remote.node_id.as_str()) {
        bail!(UPGRADE);
    }
    Ok((snapshot, detail))
}

fn remote_status_failure<T>(status: u16) -> Result<T> {
    if status == 426 {
        bail!(UPGRADE);
    }
    bail!("worker status failed with HTTP {status}")
}

fn configured_remote(access: &RemoteAccess, node_id: &str) -> Result<RemoteRecord> {
    access
        .configured()?
        .remotes
        .into_iter()
        .find(|remote| remote.enabled && remote.node_id == node_id)
        .ok_or_else(|| anyhow::anyhow!("enabled remote not found"))
}

fn target(remote: &RemoteRecord) -> crate::daemon::worker_registry::WorkerConnectionTarget {
    crate::daemon::worker_registry::WorkerConnectionTarget {
        node_id: remote.node_id.clone(),
        endpoint: remote.base_url.clone(),
        bearer_token: remote.bearer_token.clone(),
        transport: remote.transport.clone(),
    }
}
