use super::resource_admission::{ResourceAdmissionDecision, ResourceRequest, ResourceRequestInput};
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
    let mut started_seq = 1;
    let mut next_output_seq = 2;
    let admission_request = match ResourceRequest::new(ResourceRequestInput {
        idempotency_key,
        task_run_id: &execution.payload.task_run_id,
        attempt: execution.payload.attempt,
        task_label: &execution.payload.task_label,
        runtime: execution.payload.runtime.as_ref(),
        origin: execution.payload.origin.clone(),
        runtime_source: execution.payload.runtime_source.clone(),
        command: execution.payload.command.clone(),
    }) {
        Ok(request) => request,
        Err(error) => {
            tracing::error!("failed to build resource request for {idempotency_key}: {error:#}");
            return;
        }
    };
    match execution
        .context
        .admit_or_queue_resources(admission_request)
    {
        Ok(ResourceAdmissionDecision::Admitted) => {}
        Ok(ResourceAdmissionDecision::Queued { queue_position }) => {
            append_queue_event(store, idempotency_key, queue_position, started_at);
            started_seq = 2;
            next_output_seq = 3;
            if let Err(error) = execution
                .context
                .wait_until_resources_admitted(idempotency_key)
            {
                tracing::error!("resource admission wait failed for {idempotency_key}: {error:#}");
                return;
            }
        }
        Err(error) => {
            tracing::error!("resource admission failed for {idempotency_key}: {error:#}");
            return;
        }
    }
    let execution_root =
        execution_root_for_submit_key_at_base(idempotency_key, &execution.execution_root_base);
    if let Err(error) = execution.context.register_active_job(
        idempotency_key.to_string(),
        super::status_state::ActiveJobMetadata::new(super::status_state::ActiveJobMetadataInput {
            task_run_id: &execution.payload.task_run_id,
            attempt: execution.payload.attempt,
            task_label: &execution.payload.task_label,
            needs: &execution.payload.needs,
            runtime: execution
                .payload
                .runtime
                .as_ref()
                .map(|_| "containerized".to_string()),
            origin: execution.payload.origin.clone(),
            runtime_source: execution.payload.runtime_source.clone(),
            command: execution.payload.command.clone(),
            resource_limits: runtime_resource_limits(execution.payload.runtime.as_ref()),
            execution_root,
        }),
    ) {
        tracing::error!("failed to register active job for submit {idempotency_key}: {error:#}");
        let _ = execution.context.release_resources(idempotency_key);
        return;
    }
    let output_observer = Arc::new(RemoteWorkerEventObserver::new_with_next_seq(
        store.clone(),
        idempotency_key.to_string(),
        next_output_seq,
    ));
    if let Err(error) = store.append_event(
        idempotency_key,
        started_seq,
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

    let finished_at = unix_epoch_ms();
    let duration_ms = finished_at.saturating_sub(started_at);
    persist_worker_execution_result(WorkerExecutionResultPersistence {
        execution,
        output_observer,
        idempotency_key,
        started_at,
        finished_at,
        duration_ms,
    });

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
    if let Err(error) = execution.context.release_resources(idempotency_key) {
        tracing::error!("failed to release resources for submit {idempotency_key}: {error:#}");
    }
}

fn append_queue_event(
    store: &SubmitAttemptStore,
    idempotency_key: &str,
    queue_position: usize,
    timestamp_ms: i64,
) {
    let ahead = queue_position.saturating_sub(1);
    let message = format!("queued on remote node; {ahead} tasks ahead");
    if let Err(error) = store.append_event(
        idempotency_key,
        1,
        &serde_json::json!({
            "kind": "TASK_QUEUED",
            "timestamp_ms": timestamp_ms,
            "queue_position": queue_position,
            "message": message,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_QUEUED event for submit {idempotency_key}: {error:#}"
        );
    }
}

fn runtime_resource_limits(
    runtime: Option<&RemoteRuntimeSpec>,
) -> Option<tak_core::model::ContainerResourceLimitsSpec> {
    match runtime {
        Some(RemoteRuntimeSpec::Containerized {
            resource_limits, ..
        }) => resource_limits.clone(),
        None => None,
    }
}

include!("worker_submit_execution/output_observer.rs");
include!("worker_submit_execution/result_persistence.rs");
include!("worker_submit_execution/session_paths.rs");
include!("worker_submit_execution/execute_submit.rs");
include!("worker_submit_execution/execute_submit_retry.rs");
include!("worker_submit_execution/execute_submit_workspace.rs");
include!("worker_submit_execution/completion_helpers.rs");

#[cfg(test)]
mod completion_helpers_tests;
