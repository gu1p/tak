use super::*;

pub(super) fn spawn_remote_worker_submit_execution(
    store: SubmitAttemptStore,
    idempotency_key: String,
    selected_node_id: String,
    transport_kind: String,
    payload: RemoteWorkerSubmitPayload,
) {
    let thread_name = format!("takd-remote-worker-{idempotency_key}");
    let spawn_result = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            run_remote_worker_submit_execution(
                &store,
                &idempotency_key,
                &selected_node_id,
                &transport_kind,
                &payload,
            )
        });
    if let Err(error) = spawn_result {
        eprintln!("failed to spawn remote worker thread: {error}");
    }
}

fn run_remote_worker_submit_execution(
    store: &SubmitAttemptStore,
    idempotency_key: &str,
    selected_node_id: &str,
    transport_kind: &str,
    payload: &RemoteWorkerSubmitPayload,
) {
    let started_at = unix_epoch_ms();
    if let Err(error) = store.append_event(
        idempotency_key,
        1,
        &serde_json::json!({
            "kind": "TASK_STARTED",
            "timestamp_ms": started_at,
        })
        .to_string(),
    ) {
        eprintln!("failed to append TASK_STARTED event for submit {idempotency_key}: {error:#}");
    }

    let execution_result = execute_remote_worker_submit(idempotency_key, selected_node_id, payload);
    let finished_at = unix_epoch_ms();
    let duration_ms = finished_at.saturating_sub(started_at);

    match execution_result {
        Ok((result, outputs)) => {
            let terminal_kind = if result.success {
                "TASK_COMPLETED"
            } else {
                "TASK_FAILED"
            };
            let exit_code = result
                .exit_code
                .unwrap_or(if result.success { 0 } else { 1 });
            if let Err(error) = store.set_result_payload(
                idempotency_key,
                &serde_json::json!({
                    "success": result.success,
                    "exit_code": exit_code,
                    "started_at": started_at,
                    "finished_at": finished_at,
                    "duration_ms": duration_ms,
                    "transport_kind": transport_kind,
                    "sync_mode": "OUTPUTS_AND_LOGS",
                    "outputs": outputs,
                    "runtime": result.runtime_kind,
                    "runtime_engine": result.runtime_engine,
                })
                .to_string(),
            ) {
                eprintln!("failed to persist submit result {idempotency_key}: {error:#}");
            }
            if let Err(error) = store.append_event(
                idempotency_key,
                2,
                &serde_json::json!({
                    "kind": terminal_kind,
                    "timestamp_ms": finished_at,
                    "success": result.success,
                    "exit_code": exit_code,
                })
                .to_string(),
            ) {
                eprintln!(
                    "failed to append terminal event for submit {idempotency_key}: {error:#}"
                );
            }
        }
        Err(error) => {
            if let Err(persist_error) = store.set_result_payload(
                idempotency_key,
                &serde_json::json!({
                    "success": false,
                    "exit_code": 1,
                    "started_at": started_at,
                    "finished_at": finished_at,
                    "duration_ms": duration_ms,
                    "transport_kind": transport_kind,
                    "sync_mode": "OUTPUTS_AND_LOGS",
                    "outputs": serde_json::json!([]),
                    "stderr_tail": error.to_string(),
                })
                .to_string(),
            ) {
                eprintln!(
                    "failed to persist failure submit result {idempotency_key}: {persist_error:#}"
                );
            }
            if let Err(append_error) = store.append_event(
                idempotency_key,
                2,
                &serde_json::json!({
                    "kind": "TASK_FAILED",
                    "timestamp_ms": finished_at,
                    "success": false,
                    "exit_code": 1,
                    "message": error.to_string(),
                })
                .to_string(),
            ) {
                eprintln!(
                    "failed to append TASK_FAILED event for submit {idempotency_key}: {append_error:#}"
                );
            }
        }
    }
}

fn execute_remote_worker_submit(
    idempotency_key: &str,
    selected_node_id: &str,
    payload: &RemoteWorkerSubmitPayload,
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
    let result = runtime.block_on(execute_remote_worker_steps(
        &execution_root,
        &RemoteWorkerExecutionSpec {
            steps: payload.steps.clone(),
            timeout_s: payload.timeout_s,
            runtime: payload.runtime.clone(),
            node_id: selected_node_id.to_string(),
        },
    ))?;
    let outputs = changed_remote_worker_outputs(&execution_root, &before)?;

    Ok((result, outputs))
}
