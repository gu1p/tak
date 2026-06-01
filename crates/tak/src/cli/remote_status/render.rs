use super::RemoteStatusResult;

#[path = "render_dashboard.rs"]
mod dashboard;
#[path = "render_format.rs"]
mod format;
#[path = "render_test_support.rs"]
mod render_test_support;
#[path = "render_sections.rs"]
mod sections;

pub(super) use dashboard::render_dashboard;
pub(super) use format::{
    age_since, format_cpu, format_image_cache, format_memory, format_needs, format_storage,
    human_bytes,
};

pub(in crate::cli) fn render_snapshot(results: &[RemoteStatusResult]) -> String {
    render_snapshot_with_prefix(results, "")
}

pub(in crate::cli) fn render_snapshot_with_prefix(
    results: &[RemoteStatusResult],
    section_prefix: &str,
) -> String {
    let mut output = format!("{section_prefix}Nodes\n");
    for result in results {
        if let Some(peer) = &result.peer {
            output.push_str(&format!(
                "{} transport={} state={} jobs={} queue={} resources={} protocol={} heartbeat={} last_heartbeat={} last_success={} reconnects={} status={}{}\n",
                peer.node_id,
                peer.transport,
                peer.state,
                peer.active_job_count
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                peer.queue_depth
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
                peer.resource_summary.as_deref().unwrap_or("n/a"),
                peer.protocol_version.as_deref().unwrap_or("n/a"),
                peer.heartbeat_rtt_ms
                    .map(|value| format!("{value}ms"))
                    .unwrap_or_else(|| "n/a".to_string()),
                peer.last_heartbeat_ms
                    .map(age_since)
                    .unwrap_or_else(|| "never".to_string()),
                peer.last_successful_connection_ms
                    .map(age_since)
                    .unwrap_or_else(|| "never".to_string()),
                peer.reconnect_attempts,
                daemon_peer_status_label(result, peer),
                peer.last_error_summary
                    .as_deref()
                    .map(|value| format!(" detail={value}"))
                    .unwrap_or_default(),
            ));
            continue;
        }
        let transport = result
            .status
            .as_ref()
            .and_then(|status| status.node.as_ref().map(|node| node.transport.as_str()))
            .unwrap_or(result.remote.transport.as_str());
        if let Some(status) = &result.status {
            let node = status.node.as_ref();
            let state = node
                .map(|node| node.transport_state.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("ready");
            let detail = node
                .map(|node| node.transport_detail.as_str())
                .filter(|value| !value.is_empty())
                .map(|value| format!(" detail={value}"))
                .unwrap_or_default();
            output.push_str(&format!(
                "{} transport={} state={} jobs={} cpu={} ram={} storage={} tak_exec={} status=ok{}\n",
                result.remote.node_id,
                transport,
                state,
                status.active_jobs.len(),
                format_cpu(status.cpu.as_ref()),
                format_memory(status.memory.as_ref()),
                format_storage(status.storage.as_ref()),
                status
                    .storage
                    .as_ref()
                    .map(|value| human_bytes(value.tak_execution_bytes))
                    .unwrap_or_else(|| "n/a".to_string()),
                detail,
            ));
            output.push_str(&format!(
                "  image_cache={} image_cache_entries={}\n",
                format_image_cache(status.image_cache.as_ref()),
                status
                    .image_cache
                    .as_ref()
                    .map(|value| value.entry_count.to_string())
                    .unwrap_or_else(|| "n/a".to_string()),
            ));
        } else {
            output.push_str(&format!(
                "{} transport={} jobs=n/a cpu=n/a ram=n/a storage=n/a tak_exec=n/a image_cache=n/a status={}\n",
                result.remote.node_id,
                transport,
                result.error.as_deref().unwrap_or("unknown_error"),
            ));
        }
    }

    sections::push_containers_section(&mut output, results, section_prefix);
    sections::push_active_jobs_section(&mut output, results, section_prefix);
    output
}

fn daemon_peer_status_label<'a>(
    result: &'a RemoteStatusResult,
    peer: &super::DaemonPeerSnapshot,
) -> &'a str {
    if let Some(error) = result.error.as_deref() {
        return error;
    }
    if peer.state == "degraded" {
        return "degraded";
    }
    "ok"
}

#[path = "render_plain_tests.rs"]
mod render_plain_tests;
#[path = "render_tests.rs"]
mod render_tests;
