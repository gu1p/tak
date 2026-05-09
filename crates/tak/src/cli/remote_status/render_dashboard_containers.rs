use ratatui::text::Line;

use super::super::super::view::RemoteStatusView;
use super::super::{age_since, human_bytes};

pub(super) fn push_container_lines(lines: &mut Vec<Line<'static>>, view: &RemoteStatusView) {
    let mut any_containers = false;
    for result in view.completed_results() {
        let Some(status) = &result.status else {
            continue;
        };
        for job in status
            .active_jobs
            .iter()
            .filter(|job| job.runtime.as_deref() == Some("containerized"))
        {
            any_containers = true;
            lines.push(Line::from(format!(
                "{} {} attempt={} age={} exec_root={} runtime={}{}{} task_run_id={}",
                result.remote.node_id,
                job.task_label,
                job.attempt,
                age_since(job.started_at_ms),
                human_bytes(job.execution_root_bytes),
                job.runtime.as_deref().unwrap_or("none"),
                optional_field(" command=", job.command.as_deref()),
                optional_field(" source=", job.runtime_source.as_deref()),
                job.task_run_id,
            )));
        }
    }
    if !any_containers {
        lines.push(Line::from("(none)"));
    }
}

fn optional_field(label: &str, value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(|value| format!("{label}{value}"))
        .unwrap_or_default()
}
