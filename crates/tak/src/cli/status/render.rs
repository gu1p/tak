use std::time::{SystemTime, UNIX_EPOCH};

use super::daemon::LocalDaemonStatus;
use super::local::{LocalResources, LocalStatusSnapshot, LocalTask};
use crate::cli::remote_status::{RemoteStatusResult, render_remote_status_snapshot_with_prefix};

pub(super) fn render_status_snapshot(
    local: &LocalStatusSnapshot,
    remote: &[RemoteStatusResult],
) -> String {
    let mut output = render_local_snapshot(local);
    output.push('\n');
    if remote.is_empty() {
        output.push_str(
            "Remote Nodes\n(none)\n\nRemote Containers\n(none)\n\nRemote Active Jobs\n(none)\n",
        );
    } else {
        output.push_str(&render_remote_status_snapshot_with_prefix(
            remote, "Remote ",
        ));
    }
    output
}

pub(super) fn render_local_snapshot(snapshot: &LocalStatusSnapshot) -> String {
    let containers = snapshot.container_tasks();
    let mut output = String::from("Local\n");
    output.push_str(&format!(
        "local status=ok daemon={} active_tasks={} containers={} cpu={} ram={} storage={}\n",
        daemon_state(&snapshot.daemon),
        snapshot.active_tasks.len(),
        containers.len(),
        format_cpu(&snapshot.resources),
        format_memory(&snapshot.resources),
        format_storage(&snapshot.resources),
    ));
    if let LocalDaemonStatus::Unavailable { detail } = &snapshot.daemon {
        output.push_str(&format!("  daemon_detail={detail}\n"));
    }
    if let LocalDaemonStatus::Available(status) = &snapshot.daemon {
        output.push_str(&format!(
            "  leases={} pending={} limiters={}\n",
            status.active_leases,
            status.pending_requests,
            status.usage.len()
        ));
    }

    output.push_str("\nContainers\n");
    if containers.is_empty() {
        output.push_str("(none)\n");
    } else {
        for task in containers {
            output.push_str(&render_task_line("local", task));
        }
    }

    output.push_str("\nActive Jobs\n");
    if snapshot.active_tasks.is_empty() {
        output.push_str("(none)\n");
    } else {
        for task in &snapshot.active_tasks {
            output.push_str(&render_task_line("local", task));
        }
    }
    output
}

fn render_task_line(node: &str, task: &LocalTask) -> String {
    format!(
        "{} {} attempt={} age={} placement={} remote_node={} origin={} runtime={} source={} command={} task_run_id={}\n",
        node,
        task.task_label,
        task.attempt,
        age_since(task.started_at_ms),
        task.placement,
        empty_none(&task.remote_node_id),
        empty_none(&task.origin),
        empty_none(&task.runtime),
        empty_none(&task.runtime_source),
        empty_none(&task.command),
        task.task_run_id,
    )
}

fn daemon_state(status: &LocalDaemonStatus) -> &'static str {
    match status {
        LocalDaemonStatus::Available(_) => "ok",
        LocalDaemonStatus::Unavailable { .. } => "unavailable",
    }
}

fn format_cpu(resources: &LocalResources) -> String {
    match resources.cpu_percent {
        Some(percent) => format!("{percent:.1}%/{}c", resources.logical_cores),
        None => format!("n/a/{}c", resources.logical_cores),
    }
}

fn format_memory(resources: &LocalResources) -> String {
    format!(
        "{}/{}",
        human_bytes(resources.memory_used_bytes),
        human_bytes(resources.memory_total_bytes)
    )
}

fn format_storage(resources: &LocalResources) -> String {
    format!(
        "{}/{} free={}",
        human_bytes(resources.storage_used_bytes),
        human_bytes(resources.storage_total_bytes),
        human_bytes(resources.storage_available_bytes)
    )
}

fn empty_none(value: &str) -> &str {
    if value.trim().is_empty() {
        "none"
    } else {
        value
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0_usize;
    while value >= 1024.0 && unit_index + 1 < UNITS.len() {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes}{}", UNITS[unit_index])
    } else {
        format!("{value:.1}{}", UNITS[unit_index])
    }
}

fn age_since(started_at_ms: i64) -> String {
    let delta_s = unix_epoch_ms().saturating_sub(started_at_ms).max(0) / 1000;
    if delta_s >= 3600 {
        return format!("{}h{}m", delta_s / 3600, (delta_s % 3600) / 60);
    }
    if delta_s >= 60 {
        return format!("{}m{}s", delta_s / 60, delta_s % 60);
    }
    format!("{delta_s}s")
}

fn unix_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}
