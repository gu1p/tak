struct WorkerExecutionResultPersistence<'a> {
    execution: &'a RemoteWorkerSubmitExecution,
    output_observer: Arc<RemoteWorkerEventObserver>,
    idempotency_key: &'a str,
    started_at: i64,
    finished_at: i64,
    duration_ms: i64,
}

fn persist_worker_execution_result(input: WorkerExecutionResultPersistence<'_>) {
    let execution = input.execution;
    let store = &execution.store;
    let stdout_tail = input.output_observer.stdout_tail();
    let stderr_tail = input.output_observer.stderr_tail();
    let execution_result = store
        .execution_root_base_for_submit(input.idempotency_key)
        .map(|value| value.unwrap_or_else(|| execution.execution_root_base.clone()))
        .and_then(|resolved_execution_root_base| {
            execute_remote_worker_submit(
                input.idempotency_key,
                &resolved_execution_root_base,
                &execution.selected_node_id,
                execution.image_cache.as_ref(),
                &execution.payload,
                input.output_observer.clone(),
                &execution.cancellation,
            )
        });

    match execution_result {
        Ok((result, outputs)) => persist_successful_worker_result(
            &input,
            &stdout_tail,
            &stderr_tail,
            result,
            outputs,
        ),
        Err(error)
            if tak_runner::is_run_cancelled_error(&error)
                || execution.cancellation.is_cancelled() =>
        {
            persist_cancelled_result(CancelledSubmitResult {
                store,
                idempotency_key: input.idempotency_key,
                transport_kind: &execution.transport_kind,
                started_at: input.started_at,
                finished_at: input.finished_at,
                duration_ms: input.duration_ms,
                stdout_tail: &stdout_tail,
                seq: input.output_observer.claim_next_seq(),
            });
        }
        Err(error) => persist_failed_worker_result(&input, &stdout_tail, &stderr_tail, error),
    }
}

fn persist_successful_worker_result(
    input: &WorkerExecutionResultPersistence<'_>,
    stdout_tail: &str,
    stderr_tail: &str,
    result: tak_runner::RemoteWorkerExecutionResult,
    outputs: Vec<RemoteWorkerOutputRecord>,
) {
    let execution = input.execution;
    let store = &execution.store;
    let terminal_kind = if result.success {
        "TASK_COMPLETED"
    } else {
        "TASK_FAILED"
    };
    let exit_code = result.exit_code.unwrap_or(if result.success { 0 } else { 1 });
    if let Err(error) = store.set_result_payload(
        input.idempotency_key,
        &serde_json::json!({
            "success": result.success,
            "exit_code": exit_code,
            "started_at": input.started_at,
            "finished_at": input.finished_at,
            "duration_ms": input.duration_ms,
            "transport_kind": execution.transport_kind.as_str(),
            "sync_mode": "OUTPUTS_AND_LOGS",
            "outputs": outputs,
            "runtime": result.runtime_kind,
            "runtime_engine": result.runtime_engine,
            "stdout_tail": json_tail_value(stdout_tail),
            "stderr_tail": json_tail_value(stderr_tail),
        })
        .to_string(),
    ) {
        tracing::error!("failed to persist submit result {}: {error:#}", input.idempotency_key);
    }
    if let Err(error) = store.append_event(
        input.idempotency_key,
        input.output_observer.claim_next_seq(),
        &serde_json::json!({
            "kind": terminal_kind,
            "timestamp_ms": input.finished_at,
            "success": result.success,
            "exit_code": exit_code,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append terminal event for submit {}: {error:#}",
            input.idempotency_key
        );
    }
}

fn persist_failed_worker_result(
    input: &WorkerExecutionResultPersistence<'_>,
    stdout_tail: &str,
    stderr_tail: &str,
    error: anyhow::Error,
) {
    let stderr_tail = failure_stderr_tail(&error, stderr_tail);
    let execution = input.execution;
    let store = &execution.store;
    if let Err(persist_error) = store.set_result_payload(
        input.idempotency_key,
        &serde_json::json!({
            "success": false,
            "exit_code": 1,
            "started_at": input.started_at,
            "finished_at": input.finished_at,
            "duration_ms": input.duration_ms,
            "transport_kind": execution.transport_kind.as_str(),
            "sync_mode": "OUTPUTS_AND_LOGS",
            "outputs": serde_json::json!([]),
            "stdout_tail": json_tail_value(stdout_tail),
            "stderr_tail": json_tail_value(&stderr_tail),
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to persist failure submit result {}: {persist_error:#}",
            input.idempotency_key
        );
    }
    if let Err(append_error) = store.append_event(
        input.idempotency_key,
        input.output_observer.claim_next_seq(),
        &serde_json::json!({
            "kind": "TASK_FAILED",
            "timestamp_ms": input.finished_at,
            "success": false,
            "exit_code": 1,
            "message": format!("{error:#}"),
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_FAILED event for submit {}: {append_error:#}",
            input.idempotency_key
        );
    }
}
