use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use prost::Message;
use tak_proto::local_daemon::v2::{Operation, RemoteStatusEntry, Response};
use tak_proto::worker_v2::WorkerSnapshot;
use tak_proto::{CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse};

use crate::cli::remote_daemon;

use super::RemoteStatusResult;

pub(in crate::cli) async fn fetch_snapshot(
    node_filters: &[String],
) -> Result<Vec<RemoteStatusResult>> {
    let node_ids = normalized_node_ids(node_filters);
    let response =
        remote_daemon::request(Operation::GetRemoteStatus { node_ids }, "remote-status").await?;
    let Response::RemoteStatus { remotes, .. } = response else {
        bail!(
            "Local takd returned an unexpected remote status response; upgrade tak, takd, and workers together"
        )
    };
    let mut results = remotes
        .into_iter()
        .map(status_result)
        .collect::<Result<Vec<_>>>()?;
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    Ok(results)
}

fn normalized_node_ids(node_filters: &[String]) -> Vec<String> {
    node_filters
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn status_result(entry: RemoteStatusEntry) -> Result<RemoteStatusResult> {
    let RemoteStatusEntry {
        remote,
        snapshot,
        detail_base64,
        error,
        peer,
    } = entry;
    let status = match detail_base64 {
        Some(detail) => Some(decode_status_detail(&detail)?),
        None => snapshot
            .as_ref()
            .map(|snapshot| status_from_snapshot(&remote, snapshot)),
    };
    Ok(RemoteStatusResult {
        remote,
        status,
        error,
        peer,
    })
}

fn decode_status_detail(encoded: &str) -> Result<NodeStatusResponse> {
    let bytes = STANDARD
        .decode(encoded)
        .context("decode local takd remote status payload")?;
    NodeStatusResponse::decode(bytes.as_slice()).context("decode local takd remote status protobuf")
}

fn status_from_snapshot(
    remote: &super::RemoteRecord,
    snapshot: &WorkerSnapshot,
) -> NodeStatusResponse {
    let cpu_total = snapshot.capacity.cpu_millis;
    let cpu_percent =
        (cpu_total > 0).then(|| snapshot.usage.cpu_millis as f64 * 100.0 / cpu_total as f64);
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: remote.node_id.clone(),
            display_name: remote.display_name.clone(),
            base_url: remote.base_url.clone(),
            healthy: snapshot.healthy,
            pools: remote.pools.clone(),
            tags: remote.tags.clone(),
            capabilities: remote.capabilities.clone(),
            transport: remote.transport.clone(),
            transport_state: if snapshot.healthy {
                "ready"
            } else {
                "unhealthy"
            }
            .into(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: i64::try_from(snapshot.sampled_at_ms).unwrap_or(i64::MAX),
        cpu: Some(CpuUsage {
            utilization_percent: cpu_percent,
            logical_cores: u32::try_from(cpu_total.div_ceil(1000)).unwrap_or(u32::MAX),
            ..Default::default()
        }),
        memory: Some(MemoryUsage {
            used_bytes: snapshot.usage.memory_bytes,
            total_bytes: snapshot.capacity.memory_bytes,
            available_bytes: Some(
                snapshot
                    .capacity
                    .memory_bytes
                    .saturating_sub(snapshot.usage.memory_bytes),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}
