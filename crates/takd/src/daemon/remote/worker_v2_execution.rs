use sha2::{Digest, Sha256};
use tak_proto::worker_v2::{DispatchAttemptRequest, WorkerTerminalOutcome};

use super::{RemoteNodeContext, SubmitAttemptStore};

mod admission;
mod execute;
mod outputs;
mod workspace;
mod workspace_cache;

pub(super) use admission::{WorkerV2AdmissionLease, reserve_worker_v2_resources};
pub(in crate::daemon::remote) use workspace_cache::{
    WorkspaceCachePin, cached_workspace_fingerprints, pin_workspace_cache, probe_workspace_cache,
    store_workspace_cache,
};

pub(super) fn spawn_worker_v2_execution(
    context: RemoteNodeContext,
    store: SubmitAttemptStore,
    request: DispatchAttemptRequest,
    admission: WorkerV2AdmissionLease,
    workspace_pin: WorkspaceCachePin,
) {
    tokio::spawn(async move {
        let _admission = admission;
        if let Err(error) = run(&context, &store, &request, &workspace_pin).await {
            tracing::error!(
                run_id = %request.identity.run_id,
                job_id = %request.identity.job_id,
                error = %error,
                "worker v2 attempt failed"
            );
            let _ = store.discard_worker_v2_outputs(&request.identity);
            let digest = format!("{:x}", Sha256::digest(format!("failed:{error:#}")));
            let persisted = store.complete_worker_v2_attempt(
                &request.identity,
                WorkerTerminalOutcome::Failed,
                &digest,
            );
            match persisted {
                Ok(_) => cleanup_terminal_workspace(&context, &request),
                Err(error) => tracing::error!(
                    run_id = %request.identity.run_id,
                    job_id = %request.identity.job_id,
                    error = %error,
                    "failed to persist worker v2 terminal state"
                ),
            }
        }
    });
}

async fn run(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    request: &DispatchAttemptRequest,
    workspace_pin: &WorkspaceCachePin,
) -> anyhow::Result<()> {
    store.mark_worker_v2_running(&request.identity)?;
    let prepared = workspace::prepare(context, request, workspace_pin)?;
    let cancellation = tak_runner::RunCancellation::new();
    let watcher = spawn_cancellation_watcher(store.clone(), request.clone(), cancellation.clone());
    let outcome = execute::run(context, store, request, &prepared, &cancellation).await;
    watcher.abort();
    let (terminal_outcome, digest, exit_code, runtime_kind, runtime_engine) = match outcome {
        Ok(execute::ExecutionOutcome::Succeeded {
            runtime_kind,
            runtime_engine,
        }) if !cancellation.is_cancelled() => {
            let _ = prepared.publish_path_cache()?;
            (
                WorkerTerminalOutcome::Succeeded,
                format!("{:x}", Sha256::digest(b"succeeded")),
                Some(0),
                runtime_kind,
                runtime_engine,
            )
        }
        Ok(execute::ExecutionOutcome::Failed {
            exit_code,
            runtime_kind,
            runtime_engine,
        }) if !cancellation.is_cancelled() => (
            WorkerTerminalOutcome::Failed,
            format!("{:x}", Sha256::digest(format!("failed:{exit_code:?}"))),
            exit_code,
            runtime_kind,
            runtime_engine,
        ),
        Ok(_) => (
            WorkerTerminalOutcome::Cancelled,
            format!("{:x}", Sha256::digest(b"cancelled")),
            None,
            None,
            None,
        ),
        Err(error) if tak_runner::is_run_cancelled_error(&error) => (
            WorkerTerminalOutcome::Cancelled,
            format!("{:x}", Sha256::digest(b"cancelled")),
            None,
            None,
            None,
        ),
        Err(error) => return Err(error),
    };
    if terminal_outcome != WorkerTerminalOutcome::Succeeded {
        store.discard_worker_v2_outputs(&request.identity)?;
    }
    store.complete_worker_v2_attempt_with_runtime(
        &request.identity,
        terminal_outcome,
        &digest,
        exit_code,
        runtime_kind,
        runtime_engine,
    )?;
    cleanup_terminal_workspace(context, request);
    Ok(())
}

fn cleanup_terminal_workspace(context: &RemoteNodeContext, request: &DispatchAttemptRequest) {
    if let Err(error) = workspace::cleanup_attempt(context, request) {
        tracing::warn!(
            run_id = %request.identity.run_id,
            job_id = %request.identity.job_id,
            error = %error,
            "failed to clean worker v2 attempt workspace"
        );
    }
}

fn spawn_cancellation_watcher(
    store: SubmitAttemptStore,
    request: DispatchAttemptRequest,
    cancellation: tak_runner::RunCancellation,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if worker_v2_cancellation_poll_requests_cancel(
                store.worker_v2_cancellation_requested(&request.identity),
            ) {
                cancellation.cancel();
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
}

#[doc(hidden)]
pub fn worker_v2_cancellation_poll_requests_cancel(requested: anyhow::Result<bool>) -> bool {
    matches!(requested, Ok(true))
}
