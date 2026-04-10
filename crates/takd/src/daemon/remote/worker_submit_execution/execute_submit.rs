fn execute_remote_worker_submit(
    idempotency_key: &str,
    selected_node_id: &str,
    payload: &RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
) -> Result<(
    tak_runner::RemoteWorkerExecutionResult,
    Vec<RemoteWorkerOutputRecord>,
)> {
    let execution_root = execution_root_for_submit_key(idempotency_key);
    if execution_root.exists() {
        fs::remove_dir_all(&execution_root).with_context(|| {
            format!(
                "failed to clear existing remote execution root {}",
                execution_root.display()
            )
        })?;
    }
    fs::create_dir_all(&execution_root).with_context(|| {
        format!(
            "failed to create remote execution root {}",
            execution_root.display()
        )
    })?;

    unpack_remote_worker_workspace(&payload.workspace_zip, &execution_root)?;
    let before = snapshot_workspace_files(&execution_root)?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime for remote worker execution")?;
    let task_label = parse_label(&payload.task_label, "//")
        .map_err(|err| anyhow!("invalid submit task label {}: {err}", payload.task_label))?;
    let result = runtime.block_on(execute_remote_worker_steps_with_output(
        &execution_root,
        &RemoteWorkerExecutionSpec {
            task_label,
            attempt: payload.attempt,
            steps: payload.steps.clone(),
            timeout_s: payload.timeout_s,
            runtime: payload.runtime.clone(),
            node_id: selected_node_id.to_string(),
        },
        Some(output_observer),
    ))?;
    let outputs = changed_remote_worker_outputs(&execution_root, &before)?;

    Ok((result, outputs))
}
