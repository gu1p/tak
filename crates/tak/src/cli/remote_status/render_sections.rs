use super::{RemoteStatusResult, age_since, format_needs, human_bytes};

pub(super) fn push_containers_section(
    output: &mut String,
    results: &[RemoteStatusResult],
    section_prefix: &str,
) {
    output.push_str(&format!("\n{section_prefix}Containers\n"));
    let mut any_containers = false;
    for result in results {
        let Some(status) = &result.status else {
            continue;
        };
        for job in status
            .active_jobs
            .iter()
            .filter(|job| job.runtime.as_deref() == Some("containerized"))
        {
            any_containers = true;
            output.push_str(&format!(
                "{} {} attempt={} age={} needs={} exec_root={} runtime={}{}{} task_run_id={}\n",
                result.remote.node_id,
                display_job_label(&job.task_label, job.execution_label.as_deref()),
                job.attempt,
                age_since(job.started_at_ms),
                format_needs(&job.needs),
                human_bytes(job.execution_root_bytes),
                job.runtime.as_deref().unwrap_or("none"),
                optional_field(" command=", job.command.as_deref()),
                optional_field(" source=", job.runtime_source.as_deref()),
                job.task_run_id,
            ));
        }
    }
    if !any_containers {
        output.push_str("(none)\n");
    }
}

pub(super) fn push_active_jobs_section(
    output: &mut String,
    results: &[RemoteStatusResult],
    section_prefix: &str,
) {
    output.push_str(&format!("\n{section_prefix}Active Jobs\n"));
    let mut any_jobs = false;
    for result in results {
        let Some(status) = &result.status else {
            continue;
        };
        for job in &status.active_jobs {
            any_jobs = true;
            output.push_str(&format!(
                "{} {} attempt={} age={} needs={} exec_root={} runtime={}{}{}\n",
                result.remote.node_id,
                display_job_label(&job.task_label, job.execution_label.as_deref()),
                job.attempt,
                age_since(job.started_at_ms),
                format_needs(&job.needs),
                human_bytes(job.execution_root_bytes),
                job.runtime.as_deref().unwrap_or("none"),
                optional_field(" command=", job.command.as_deref()),
                optional_field(" source=", job.runtime_source.as_deref()),
            ));
        }
    }
    if !any_jobs {
        output.push_str("(none)\n");
    }
}

fn optional_field(label: &str, value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(|value| format!("{label}{value}"))
        .unwrap_or_default()
}

fn display_job_label<'a>(task_label: &'a str, execution_label: Option<&'a str>) -> &'a str {
    execution_label
        .filter(|label| !label.trim().is_empty())
        .unwrap_or(task_label)
}
