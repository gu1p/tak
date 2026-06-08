use std::time::{SystemTime, UNIX_EPOCH};

use tak_proto::{NodeStatusResponse, SubmittedNeed};

pub(super) fn render_active_tasks(status: &NodeStatusResponse) -> String {
    let node_id = status
        .node
        .as_ref()
        .map(|node| node.node_id.as_str())
        .unwrap_or("unknown");
    let mut output = String::from("Active Tasks\n");
    if status.active_jobs.is_empty() {
        output.push_str("(none)\n");
        return output;
    }
    for job in &status.active_jobs {
        output.push_str(&format!(
            "node={} task_label={} task_run_id={} attempt={} age={} needs={} exec_root={} runtime={}\n",
            node_id,
            task_label_or_unknown(display_task_label(&job.task_label, job.execution_label.as_deref())),
            job.task_run_id,
            job.attempt,
            age_since(job.started_at_ms),
            format_needs(&job.needs),
            human_bytes(job.execution_root_bytes),
            job.runtime.as_deref().unwrap_or("none"),
        ));
    }
    output
}

fn task_label_or_unknown(label: &str) -> &str {
    if label.trim().is_empty() {
        "(unknown)"
    } else {
        label
    }
}

fn display_task_label<'a>(task_label: &'a str, execution_label: Option<&'a str>) -> &'a str {
    execution_label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or(task_label)
}

fn format_needs(needs: &[SubmittedNeed]) -> String {
    if needs.is_empty() {
        return "(none)".to_string();
    }
    needs.iter().map(format_need).collect::<Vec<_>>().join(",")
}

fn format_need(need: &SubmittedNeed) -> String {
    let scope_key = need
        .scope_key
        .as_deref()
        .map(|value| format!("/{value}"))
        .unwrap_or_default();
    format!("{}({}{})={}", need.name, need.scope, scope_key, need.slots)
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
