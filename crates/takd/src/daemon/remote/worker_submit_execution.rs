use super::*;
use std::sync::Arc;

pub(super) struct RemoteWorkerSubmitExecution {
    pub(super) store: SubmitAttemptStore,
    pub(super) context: RemoteNodeContext,
    pub(super) idempotency_key: String,
    pub(super) execution_root_base: std::path::PathBuf,
    pub(super) selected_node_id: String,
    pub(super) transport_kind: String,
    pub(super) image_cache: Option<super::types::RemoteImageCacheRuntimeConfig>,
    pub(super) cancellation: tak_runner::RunCancellation,
    pub(super) payload: RemoteWorkerSubmitPayload,
}

pub(super) fn spawn_remote_worker_submit_execution(
    execution: RemoteWorkerSubmitExecution,
) -> Result<()> {
    let thread_name = format!("takd-remote-worker-{}", execution.idempotency_key);
    std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || run_remote_worker_submit_execution(&execution))
        .context("failed to spawn remote worker thread")?;
    Ok(())
}

fn run_remote_worker_submit_execution(execution: &RemoteWorkerSubmitExecution) {
    let store = &execution.store;
    let idempotency_key = execution.idempotency_key.as_str();
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
        .map(|value| value.unwrap_or_else(|| execution.execution_root_base.clone()))
        .and_then(|resolved_execution_root_base| {
            execute_remote_worker_submit(
                idempotency_key,
                &resolved_execution_root_base,
                &execution.selected_node_id,
                execution.image_cache.as_ref(),
                &execution.payload,
                output_observer.clone(),
                &execution.cancellation,
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
                    "transport_kind": execution.transport_kind.as_str(),
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
        Err(error)
            if tak_runner::is_run_cancelled_error(&error)
                || execution.cancellation.is_cancelled() =>
        {
            persist_cancelled_result(CancelledSubmitResult {
                store,
                idempotency_key,
                transport_kind: &execution.transport_kind,
                started_at,
                finished_at,
                duration_ms,
                stdout_tail: &stdout_tail,
                seq: output_observer.claim_next_seq(),
            });
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
                    "transport_kind": execution.transport_kind.as_str(),
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
                    "message": format!("{error:#}"),
                })
                .to_string(),
            ) {
                tracing::error!(
                    "failed to append TASK_FAILED event for submit {idempotency_key}: {append_error:#}"
                );
            }
        }
    }

    if let Err(error) = execution.context.finish_active_job(idempotency_key) {
        tracing::error!(
            "failed to clear active node status entry for submit {idempotency_key}: {error:#}"
        );
    }
    if let Err(error) = execution
        .context
        .unregister_active_execution(idempotency_key)
    {
        tracing::error!(
            "failed to unregister active execution for submit {idempotency_key}: {error:#}"
        );
    }
}

struct CancelledSubmitResult<'a> {
    store: &'a SubmitAttemptStore,
    idempotency_key: &'a str,
    transport_kind: &'a str,
    started_at: i64,
    finished_at: i64,
    duration_ms: i64,
    stdout_tail: &'a str,
    seq: u64,
}

fn persist_cancelled_result(result: CancelledSubmitResult<'_>) {
    let stderr_tail = "task cancelled";
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
            "stderr_tail": stderr_tail,
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
            "message": stderr_tail,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_CANCELLED event for submit {}: {error:#}",
            result.idempotency_key
        );
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

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::failure_stderr_tail;

    #[test]
    fn failure_stderr_tail_preserves_error_chain() {
        let error = anyhow!("docker build error: package index fetch failed")
            .context("infra error: container lifecycle build failed");

        let tail = failure_stderr_tail(&error, "");

        assert!(tail.contains("infra error: container lifecycle build failed"));
        assert!(tail.contains("docker build error: package index fetch failed"));
    }

    #[test]
    fn failure_stderr_tail_prepends_error_chain_to_existing_stderr() {
        let error = anyhow!("docker build error: package index fetch failed")
            .context("infra error: container lifecycle build failed");

        let tail = failure_stderr_tail(&error, "existing stderr\n");

        assert!(tail.starts_with("infra error: container lifecycle build failed"));
        assert!(tail.contains("docker build error: package index fetch failed"));
        assert!(tail.ends_with("existing stderr\n"));
    }
}

include!("worker_submit_execution/output_observer.rs");
include!("worker_submit_execution/session_paths.rs");
include!("worker_submit_execution/execute_submit.rs");
