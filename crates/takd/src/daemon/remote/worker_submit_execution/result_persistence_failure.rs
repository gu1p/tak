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
            "failure_kind": "infrastructure",
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
