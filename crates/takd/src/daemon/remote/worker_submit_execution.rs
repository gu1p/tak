use super::*;
use std::sync::Arc;

#[derive(Clone, Copy)]
pub(super) enum PreparedResourceAdmission {
    Admitted {
        started_at: i64,
    },
    Queued {
        queue_position: usize,
        queued_at_ms: i64,
    },
}

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
    pub(super) admission: PreparedResourceAdmission,
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
    let mut started_seq = 1;
    let mut next_output_seq = 2;
    let started_at = match execution.admission {
        PreparedResourceAdmission::Admitted { started_at } => started_at,
        PreparedResourceAdmission::Queued {
            queue_position,
            queued_at_ms,
        } => {
            append_queue_event(store, idempotency_key, queue_position, queued_at_ms);
            started_seq = 2;
            next_output_seq = 3;
            if let Err(error) = execution
                .context
                .wait_until_resources_admitted(idempotency_key, &execution.cancellation)
            {
                if tak_runner::is_run_cancelled_error(&error)
                    || execution.cancellation.is_cancelled()
                {
                    persist_queued_cancelled_submit(execution, idempotency_key, queued_at_ms);
                } else {
                    tracing::error!(
                        "resource admission wait failed for {idempotency_key}: {error:#}"
                    );
                    persist_queued_failed_submit(execution, idempotency_key, queued_at_ms, error);
                }
                finish_queued_submit_without_run(execution, idempotency_key);
                return;
            }
            let started_at = unix_epoch_ms();
            if let Err(error) = register_active_job_for_submit(execution, started_at) {
                tracing::error!(
                    "failed to register active job for submit {idempotency_key}: {error:#}"
                );
                persist_queued_failed_submit(execution, idempotency_key, started_at, error);
                finish_queued_submit_without_run(execution, idempotency_key);
                return;
            }
            started_at
        }
    };
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

    let execution_result = store
        .execution_root_base_for_submit(idempotency_key)
        .map(|value| value.unwrap_or_else(|| execution.execution_root_base.clone()))
        .and_then(|resolved_execution_root_base| {
            execute_remote_worker_submit(RemoteWorkerSubmitRunContext {
                idempotency_key,
                execution_root_base: &resolved_execution_root_base,
                selected_node_id: &execution.selected_node_id,
                image_cache: execution.image_cache.as_ref(),
                payload: &execution.payload,
                output_observer: output_observer.clone(),
                cancellation: &execution.cancellation,
                status_context: Some(RemoteMemberStatusContext {
                    context: execution.context.clone(),
                    idempotency_key: idempotency_key.to_string(),
                }),
            })
        });
    let finished_at = unix_epoch_ms();
    let duration_ms = finished_at.saturating_sub(started_at);
    persist_worker_execution_result(
        WorkerExecutionResultPersistence {
            execution,
            output_observer,
            idempotency_key,
            started_at,
            finished_at,
            duration_ms,
        },
        execution_result,
    );

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

include!("worker_submit_execution/submit_status.rs");
include!("worker_submit_execution/output_observer.rs");
include!("worker_submit_execution/result_persistence.rs");
include!("worker_submit_execution/session_paths.rs");
include!("worker_submit_execution/queued_terminal.rs");
include!("worker_submit_execution/member_status.rs");
include!("worker_submit_execution/execute_context.rs");
include!("worker_submit_execution/execute_submit.rs");
include!("worker_submit_execution/execute_submit_retry.rs");
include!("worker_submit_execution/execute_submit_workspace.rs");
include!("worker_submit_execution/completion_helpers.rs");

mod completion_helpers_tests;

#[path = "worker_submit_execution/queued_failure_tests.rs"]
mod queued_failure_tests;
