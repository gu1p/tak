struct WorkerExecutionResultPersistence<'a> {
    execution: &'a RemoteWorkerSubmitExecution,
    output_observer: Arc<RemoteWorkerEventObserver>,
    idempotency_key: &'a str,
    started_at: i64,
    finished_at: i64,
    duration_ms: i64,
}

fn persist_worker_execution_result(
    input: WorkerExecutionResultPersistence<'_>,
    execution_result: Result<(tak_runner::RemoteWorkerExecutionResult, Vec<RemoteWorkerOutputRecord>)>,
) {
    let execution = input.execution;
    let stdout_tail = input.output_observer.stdout_tail();
    let stderr_tail = input.output_observer.stderr_tail();

    // A cancelled run is authoritative even if the container still produced a
    // result in the same window. The orphan watchdog (and explicit cancel) can
    // trip the shared cancellation latch while the container is racing to an
    // exit; that must be recorded as cancelled, not as a spurious
    // success/failure. The latch stays set once tripped, so checking it here
    // covers both the `Ok` and `Err` outcomes.
    if execution.cancellation.is_cancelled() {
        persist_cancelled_worker_result(&input, &stdout_tail);
        return;
    }

    match execution_result {
        Ok((result, outputs)) => {
            persist_successful_worker_result(&input, &stdout_tail, &stderr_tail, result, outputs)
        }
        Err(error) if tak_runner::is_run_cancelled_error(&error) => {
            persist_cancelled_worker_result(&input, &stdout_tail)
        }
        Err(error) => persist_failed_worker_result(&input, &stdout_tail, &stderr_tail, error),
    }
}

fn persist_cancelled_worker_result(input: &WorkerExecutionResultPersistence<'_>, stdout_tail: &str) {
    let reason = input
        .execution
        .context
        .active_execution_cancel_reason(
            &input.execution.payload.task_run_id,
            Some(input.execution.payload.attempt),
        )
        .unwrap_or(None);
    tracing::warn!(
        idempotency_key = input.idempotency_key,
        task_run_id = %input.execution.payload.task_run_id,
        attempt = input.execution.payload.attempt,
        task_label = %input.execution.payload.task_label,
        duration_ms = input.duration_ms,
        "remote worker task cancelled"
    );
    persist_cancelled_result(CancelledSubmitResult {
        store: &input.execution.store,
        idempotency_key: input.idempotency_key,
        transport_kind: &input.execution.transport_kind,
        started_at: input.started_at,
        finished_at: input.finished_at,
        duration_ms: input.duration_ms,
        stdout_tail,
        stderr_tail: cancellation_message(reason),
        seq: input.output_observer.claim_next_seq(),
    });
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
    tracing::info!(
        idempotency_key = input.idempotency_key,
        task_run_id = %execution.payload.task_run_id,
        attempt = execution.payload.attempt,
        task_label = %execution.payload.task_label,
        success = result.success,
        exit_code,
        output_count = outputs.len(),
        duration_ms = input.duration_ms,
        "remote worker task finished"
    );
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
    tracing::warn!(
        idempotency_key = input.idempotency_key,
        task_run_id = %execution.payload.task_run_id,
        attempt = execution.payload.attempt,
        task_label = %execution.payload.task_label,
        duration_ms = input.duration_ms,
        error = %format!("{error:#}"),
        "remote worker task failed"
    );
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
