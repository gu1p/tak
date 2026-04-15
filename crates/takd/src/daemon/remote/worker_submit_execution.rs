use super::*;
use std::sync::Arc;

pub(super) fn spawn_remote_worker_submit_execution(
    store: SubmitAttemptStore,
    status_state: status_state::SharedNodeStatusState,
    idempotency_key: String,
    selected_node_id: String,
    transport_kind: String,
    payload: RemoteWorkerSubmitPayload,
) -> Result<()> {
    let thread_name = format!("takd-remote-worker-{idempotency_key}");
    std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            run_remote_worker_submit_execution(
                &store,
                &status_state,
                &idempotency_key,
                &selected_node_id,
                &transport_kind,
                &payload,
            )
        })
        .context("failed to spawn remote worker thread")?;
    Ok(())
}

fn run_remote_worker_submit_execution(
    store: &SubmitAttemptStore,
    status_state: &status_state::SharedNodeStatusState,
    idempotency_key: &str,
    selected_node_id: &str,
    transport_kind: &str,
    payload: &RemoteWorkerSubmitPayload,
) {
    let started_at = unix_epoch_ms();
    let output_observer = Arc::new(RemoteWorkerEventObserver::new(
        store.clone(),
        idempotency_key.to_string(),
    ));
    if let Err(error) = store.append_event(
        idempotency_key,
        1,
        &serde_json::json!({
            "kind": "TASK_STARTED",
            "timestamp_ms": started_at,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_STARTED event for submit {idempotency_key}: {error:#}"
        );
    }

    let execution_result = store
        .execution_root_base_for_submit(idempotency_key)
        .map(|value| value.unwrap_or_else(remote_execution_root_base))
        .and_then(|execution_root_base| {
            execute_remote_worker_submit(
                idempotency_key,
                &execution_root_base,
                selected_node_id,
                payload,
                output_observer.clone(),
            )
        });
    let finished_at = unix_epoch_ms();
    let duration_ms = finished_at.saturating_sub(started_at);
    let stdout_tail = output_observer.stdout_tail();
    let stderr_tail = output_observer.stderr_tail();

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
                    "stdout_tail": json_tail_value(&stdout_tail),
                    "stderr_tail": json_tail_value(&stderr_tail),
                })
                .to_string(),
            ) {
                tracing::error!("failed to persist submit result {idempotency_key}: {error:#}");
            }
            if let Err(error) = store.append_event(
                idempotency_key,
                output_observer.claim_next_seq(),
                &serde_json::json!({
                    "kind": terminal_kind,
                    "timestamp_ms": finished_at,
                    "success": result.success,
                    "exit_code": exit_code,
                })
                .to_string(),
            ) {
                tracing::error!(
                    "failed to append terminal event for submit {idempotency_key}: {error:#}"
                );
            }
        }
        Err(error) => {
            let stderr_tail = failure_stderr_tail(&error, &stderr_tail);
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
                    "stdout_tail": json_tail_value(&stdout_tail),
                    "stderr_tail": json_tail_value(&stderr_tail),
                })
                .to_string(),
            ) {
                tracing::error!(
                    "failed to persist failure submit result {idempotency_key}: {persist_error:#}"
                );
            }
            if let Err(append_error) = store.append_event(
                idempotency_key,
                output_observer.claim_next_seq(),
                &serde_json::json!({
                    "kind": "TASK_FAILED",
                    "timestamp_ms": finished_at,
                    "success": false,
                    "exit_code": 1,
                    "message": error.to_string(),
                })
                .to_string(),
            ) {
                tracing::error!(
                    "failed to append TASK_FAILED event for submit {idempotency_key}: {append_error:#}"
                );
            }
        }
    }

    if let Ok(mut guard) = status_state.lock() {
        guard.finish_job(idempotency_key);
    } else {
        tracing::error!("failed to clear active node status entry for submit {idempotency_key}");
    }
}

fn failure_stderr_tail(error: &anyhow::Error, stderr_tail: &str) -> String {
    let error_message = error.to_string();
    if stderr_tail.is_empty() || stderr_tail.contains(&error_message) {
        return if stderr_tail.is_empty() {
            error_message
        } else {
            stderr_tail.to_string()
        };
    }
    format!("{error_message}\n{stderr_tail}")
}

include!("worker_submit_execution/output_observer.rs");
include!("worker_submit_execution/execute_submit.rs");
