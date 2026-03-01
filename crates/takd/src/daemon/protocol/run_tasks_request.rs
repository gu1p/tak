use super::*;

pub(super) struct RunTaskResultEnvelope {
    pub(super) label: String,
    pub(super) attempts: u32,
    pub(super) success: bool,
    pub(super) exit_code: Option<i32>,
    pub(super) placement: String,
    pub(super) remote_node: Option<String>,
    pub(super) transport: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) context_hash: Option<String>,
    pub(super) runtime: Option<String>,
    pub(super) runtime_engine: Option<String>,
}

pub(super) async fn execute_run_tasks_request(
    payload: &RunTasksRequest,
) -> Result<Vec<RunTaskResultEnvelope>> {
    let workspace_root = PathBuf::from(payload.workspace_root.trim());
    if payload.labels.is_empty() {
        bail!("run requires at least one label");
    }

    let spec = load_workspace(&workspace_root, &LoadOptions::default())?;
    let mut targets = Vec::with_capacity(payload.labels.len());
    for raw_label in &payload.labels {
        let parsed = parse_label(raw_label, "//")
            .map_err(|err| anyhow!("invalid label {raw_label}: {err}"))?;
        targets.push(parsed);
    }

    let run_options = RunOptions {
        jobs: payload.jobs,
        keep_going: payload.keep_going,
        lease_socket: payload.lease_socket.as_ref().map(PathBuf::from),
        lease_ttl_ms: payload.lease_ttl_ms,
        lease_poll_interval_ms: payload.lease_poll_interval_ms,
        session_id: payload.session_id.clone(),
        user: payload.user.clone(),
    };
    let summary = run_tasks(&spec, &targets, &run_options).await?;

    let mut task_results = Vec::new();
    for (label, result) in summary.results {
        task_results.push(RunTaskResultEnvelope {
            label: label.to_string(),
            attempts: result.attempts,
            success: result.success,
            exit_code: result.exit_code,
            placement: result.placement_mode.as_str().to_string(),
            remote_node: result.remote_node_id,
            transport: result.remote_transport_kind,
            reason: result.decision_reason,
            context_hash: result.context_manifest_hash,
            runtime: result.remote_runtime_kind,
            runtime_engine: result.remote_runtime_engine,
        });
    }
    Ok(task_results)
}
