use std::process::ExitCode;

use anyhow::{Result, bail};

use super::super::remote_status::{RemoteStatusResult, fetch_remote_status_snapshot};
use super::super::task_history::TaskHistoryStore;
use super::DockerCliSelectors;
use super::selectors::matching_remotes;

#[derive(Debug)]
struct DockerPsRow {
    node: String,
    kind: String,
    task_label: String,
    task_run_id: String,
    attempt: u32,
    started_at_ms: i64,
    runtime: String,
    source: String,
    command: String,
}

pub(super) async fn run_docker_ps(
    selectors: DockerCliSelectors,
    args: &[String],
) -> Result<ExitCode> {
    if !args.is_empty() {
        bail!("tak docker ps does not support Docker flags yet");
    }

    let mut rows = Vec::new();
    let mut local_history_error = None;
    if should_include_local_ps(&selectors) {
        match local_ps_rows() {
            Ok(local_rows) => rows.extend(local_rows),
            Err(err) => local_history_error = Some(single_line(&format!("{err:#}"))),
        }
    }
    if !selectors.local {
        let remotes = matching_remotes(&selectors)?;
        let snapshot = fetch_remote_status_snapshot(&remotes).await;
        rows.extend(remote_ps_rows(&snapshot));
        for result in snapshot.iter().filter(|result| result.error.is_some()) {
            eprintln!(
                "warning: remote node {} unavailable: {}",
                result.remote.node_id,
                result.error.as_deref().unwrap_or("unknown error")
            );
        }
    }

    print!("{}", render_docker_ps(rows, local_history_error.as_deref()));
    Ok(ExitCode::SUCCESS)
}

fn local_ps_rows() -> Result<Vec<DockerPsRow>> {
    let rows = TaskHistoryStore::open_default()?.active_container_runs()?;
    Ok(rows
        .into_iter()
        .map(|row| DockerPsRow {
            node: "local".to_string(),
            kind: normalize_ps_kind(&row.origin, &row.task_label),
            task_label: row.task_label,
            task_run_id: row.task_run_id,
            attempt: row.attempts,
            started_at_ms: row.started_at_ms,
            runtime: empty_as_none(row.runtime),
            source: empty_as_none(row.runtime_source),
            command: empty_as_none(row.command),
        })
        .collect())
}

fn remote_ps_rows(snapshot: &[RemoteStatusResult]) -> Vec<DockerPsRow> {
    snapshot
        .iter()
        .filter_map(|result| {
            let node = result.remote.node_id.clone();
            result.status.as_ref().map(|status| {
                status
                    .active_jobs
                    .iter()
                    .filter(|job| job.runtime.as_deref() == Some("containerized"))
                    .map(|job| DockerPsRow {
                        node: node.clone(),
                        kind: normalize_ps_kind(
                            job.origin.as_deref().unwrap_or("task"),
                            &job.task_label,
                        ),
                        task_label: job.task_label.clone(),
                        task_run_id: job.task_run_id.clone(),
                        attempt: job.attempt,
                        started_at_ms: job.started_at_ms,
                        runtime: job.runtime.clone().unwrap_or_else(|| "none".to_string()),
                        source: job
                            .runtime_source
                            .clone()
                            .unwrap_or_else(|| "none".to_string()),
                        command: job.command.clone().unwrap_or_else(|| "none".to_string()),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect()
}

fn normalize_ps_kind(origin: &str, task_label: &str) -> String {
    match origin {
        "docker-run" | "exec" | "task" => origin.to_string(),
        "" if task_label == "//:docker-run" => "docker-run".to_string(),
        "" if task_label == "//:exec" => "exec".to_string(),
        _ => "task".to_string(),
    }
}

fn empty_as_none(value: String) -> String {
    if value.is_empty() {
        "none".to_string()
    } else {
        value
    }
}

fn render_docker_ps(mut rows: Vec<DockerPsRow>, local_history_error: Option<&str>) -> String {
    rows.sort_unstable_by(|left, right| {
        left.node
            .cmp(&right.node)
            .then(left.kind.cmp(&right.kind))
            .then(left.task_label.cmp(&right.task_label))
            .then(left.task_run_id.cmp(&right.task_run_id))
    });
    let mut output = String::from("Tak Containers\n");
    if let Some(detail) = local_history_error {
        output.push_str(&format!("local history=unavailable detail={detail}\n"));
    }
    if rows.is_empty() {
        output.push_str("(none)\n");
        return output;
    }
    for row in rows {
        output.push_str(&format!(
            "node={} kind={} task_label={} task_run_id={} attempt={} age={} runtime={} source={} command={}\n",
            row.node,
            row.kind,
            row.task_label,
            row.task_run_id,
            row.attempt,
            age_since(row.started_at_ms),
            row.runtime,
            row.source,
            row.command,
        ));
    }
    output
}

fn should_include_local_ps(selectors: &DockerCliSelectors) -> bool {
    selectors.local
        || (selectors.node.is_none()
            && selectors.arch.is_none()
            && selectors.os.is_none()
            && selectors.pool.is_none()
            && selectors.tags.is_empty()
            && selectors.capabilities.is_empty()
            && selectors
                .transport
                .as_deref()
                .is_none_or(|transport| transport == "any"))
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

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn unix_epoch_ms() -> i64 {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    i64::try_from(millis).unwrap_or(i64::MAX)
}
