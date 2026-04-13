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

    let execution_result = (|| -> Result<_> {
        unpack_remote_worker_workspace(&payload.workspace_zip, &execution_root)?;

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
        let outputs = collect_declared_remote_worker_outputs(
            &execution_root,
            &payload.outputs,
            result.success,
        )?;
        stage_remote_worker_outputs(idempotency_key, &execution_root, &outputs)?;

        Ok((result, outputs))
    })();

    let cleanup_result = match fs::remove_dir_all(&execution_root) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| {
            format!(
                "failed to remove remote execution root {}",
                execution_root.display()
            )
        }),
    };

    match (execution_result, cleanup_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(err)) => Err(err),
        (Err(err), Ok(())) => Err(err),
        (Err(err), Err(cleanup_err)) => Err(err.context(cleanup_err.to_string())),
    }
}
