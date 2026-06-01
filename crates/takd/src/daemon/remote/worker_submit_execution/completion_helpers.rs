struct CancelledSubmitResult<'a> {
    store: &'a SubmitAttemptStore,
    idempotency_key: &'a str,
    transport_kind: &'a str,
    started_at: i64,
    finished_at: i64,
    duration_ms: i64,
    stdout_tail: &'a str,
    stderr_tail: String,
    seq: u64,
}

fn persist_cancelled_result(result: CancelledSubmitResult<'_>) {
    if let Err(error) = result.store.set_result_payload(
        result.idempotency_key,
        &serde_json::json!({
            "success": false,
            "status": "cancelled",
            "exit_code": serde_json::Value::Null,
            "started_at": result.started_at,
            "finished_at": result.finished_at,
            "duration_ms": result.duration_ms,
            "transport_kind": result.transport_kind,
            "sync_mode": "OUTPUTS_AND_LOGS",
            "outputs": serde_json::json!([]),
            "stdout_tail": json_tail_value(result.stdout_tail),
            "stderr_tail": result.stderr_tail,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to persist cancelled submit result {}: {error:#}",
            result.idempotency_key
        );
    }
    if let Err(error) = result.store.append_event(
        result.idempotency_key,
        result.seq,
        &serde_json::json!({
            "kind": "TASK_CANCELLED",
            "timestamp_ms": result.finished_at,
            "success": false,
            "message": result.stderr_tail,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_CANCELLED event for submit {}: {error:#}",
            result.idempotency_key
        );
    }
}

fn cancellation_message(
    reason: Option<super::active_executions::ActiveExecutionCancelReason>,
) -> String {
    match reason {
        Some(super::active_executions::ActiveExecutionCancelReason::ClientStale { stale_ms }) => format!(
            "remote worker cancelled task after {}s without client contact (client presumed disconnected)",
            stale_ms.saturating_add(999) / 1000
        ),
        _ => "task cancelled".to_string(),
    }
}

fn failure_stderr_tail(error: &anyhow::Error, stderr_tail: &str) -> String {
    let error_message = format!("{error:#}");
    if stderr_tail.is_empty() || stderr_tail.contains(&error_message) {
        return if stderr_tail.is_empty() {
            error_message
        } else {
            stderr_tail.to_string()
        };
    }
    format!("{error_message}\n{stderr_tail}")
}
