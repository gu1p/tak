fn persist_queued_cancelled_submit(
    execution: &RemoteWorkerSubmitExecution,
    idempotency_key: &str,
    queued_at_ms: i64,
) {
    let finished_at = unix_epoch_ms();
    let reason = execution
        .context
        .active_execution_cancel_reason(&execution.payload.task_run_id, Some(execution.payload.attempt))
        .unwrap_or(None);
    persist_cancelled_result(CancelledSubmitResult {
        store: &execution.store,
        idempotency_key,
        transport_kind: &execution.transport_kind,
        started_at: queued_at_ms,
        finished_at,
        duration_ms: finished_at.saturating_sub(queued_at_ms),
        stdout_tail: "",
        stderr_tail: cancellation_message(reason),
        seq: 2,
    });
}

fn persist_queued_failed_submit(
    execution: &RemoteWorkerSubmitExecution,
    idempotency_key: &str,
    started_at: i64,
    error: anyhow::Error,
) {
    let finished_at = unix_epoch_ms();
    let message = format!("{error:#}");
    if let Err(error) = execution.store.set_result_payload(
        idempotency_key,
        &serde_json::json!({
            "success": false,
            "exit_code": 1,
            "started_at": started_at,
            "finished_at": finished_at,
            "duration_ms": finished_at.saturating_sub(started_at),
            "transport_kind": execution.transport_kind.as_str(),
            "sync_mode": "OUTPUTS_AND_LOGS",
            "outputs": serde_json::json!([]),
            "stdout_tail": serde_json::Value::Null,
            "stderr_tail": message,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to persist queued failure submit result {idempotency_key}: {error:#}"
        );
    }
    if let Err(error) = execution.store.append_event(
        idempotency_key,
        2,
        &serde_json::json!({
            "kind": "TASK_FAILED",
            "timestamp_ms": finished_at,
            "success": false,
            "exit_code": 1,
            "message": message,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append queued TASK_FAILED event for submit {idempotency_key}: {error:#}"
        );
    }
}

fn finish_queued_submit_without_run(
    execution: &RemoteWorkerSubmitExecution,
    idempotency_key: &str,
) {
    if let Err(error) = execution
        .context
        .unregister_active_execution(idempotency_key)
    {
        tracing::error!(
            "failed to unregister active execution for queued submit {idempotency_key}: {error:#}"
        );
    }
    if let Err(error) = execution.context.release_resources(idempotency_key) {
        tracing::error!(
            "failed to release queued resources for submit {idempotency_key}: {error:#}"
        );
    }
}
