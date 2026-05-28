use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

mod attempt_execution;
mod attempt_placement;
mod attempt_submit;
mod cancellation;
mod execution_labels;
mod fused_cascade;
mod fused_cascade_run;
mod lease_context;
mod output_observer;
mod placement;
mod placement_remote;
#[cfg(test)]
mod placement_remote_tests;
mod placement_session;
mod preflight_capacity;
pub(crate) mod preflight_failure;
pub(crate) mod preflight_fallback;
mod preflight_status_output;
mod protocol_cancel;
mod protocol_detection;
mod protocol_events;
pub(crate) mod protocol_result_http;
mod protocol_submit;
#[path = "protocol_submit_tests.rs"]
mod protocol_submit_tests;
mod public_types;
pub(crate) mod remote_diagnostics;
mod remote_http_exchange_error;
pub(crate) mod remote_models;
mod remote_selection;
#[path = "remote_selection_reservation_tests.rs"]
mod remote_selection_reservation_tests;
#[path = "remote_selection_tests.rs"]
mod remote_selection_tests;
pub(crate) mod remote_submit_failure;
mod remote_wait_status;
mod remote_worker;
mod result_tail_recovery;
mod run_single_task;
mod run_tasks;
mod runtime_metadata;
mod session_cascade;
mod session_cascade_context;
mod session_cascade_selection;
mod session_tempdir;
mod session_workspace_files;
pub(crate) mod session_workspaces;
mod step_execution;
mod task_result;
mod transport;
#[path = "transport_tests.rs"]
mod transport_tests;
mod transport_tor;
mod workspace_collect;
mod workspace_outputs;
mod workspace_stage;
mod workspace_sync;
#[path = "workspace_sync_test_support.rs"]
mod workspace_sync_test_support;
#[path = "workspace_sync_tests.rs"]
mod workspace_sync_tests;
mod workspace_upload;
#[path = "workspace_upload_auth_tests.rs"]
mod workspace_upload_auth_tests;
#[path = "workspace_upload_raw_http_test_support.rs"]
mod workspace_upload_raw_http_test_support;
#[path = "workspace_upload_test_support.rs"]
mod workspace_upload_test_support;
#[path = "workspace_upload_tests.rs"]
mod workspace_upload_tests;

pub use cancellation::{RunCancellation, RunCancelled, is_run_cancelled_error};
pub use public_types::{
    ContainerExecutionIdentity, ImageCacheOptions, OutputStream, PlacementMode, RemoteLogChunk,
    RemoteWorkerExecutionResult, RemoteWorkerExecutionSpec, RunOptions, RunSummary, SyncedOutput,
    TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskRunResult, TaskStartedEvent,
    TaskStatusEvent, TaskStatusPhase,
};
pub use remote_diagnostics::{
    NoMatchingRemoteError, RemoteCandidateDiagnostic, RemoteCandidateRejection,
    RemotePreflightExhaustedError, RemotePreflightFailure, RemotePreflightFailureKind,
    RequiredRemoteDiagnostic,
};
pub use remote_worker::{
    execute_remote_worker_steps, execute_remote_worker_steps_with_cancellation,
    execute_remote_worker_steps_with_output,
    execute_remote_worker_steps_with_output_and_cancellation,
};
pub use run_tasks::run_tasks;

pub(crate) use cancellation::cancelled_error;
pub(crate) use lease_context::LeaseContext;
pub(crate) use output_observer::{emit_task_finished, emit_task_output, emit_task_started};
pub(crate) use remote_http_exchange_error::{RemoteHttpExchangeError, RemoteHttpExchangeErrorKind};
pub(crate) use remote_models::{
    ContainerExecutionPlan, ImageCachePlan, ParsedRemoteEvents, RemoteTargetSelection,
    RemoteWorkspaceStage, StrictRemoteTarget,
};

/// Executes exactly one resolved task and preserves the task's own success and exit metadata.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn demo(task: &tak_core::model::ResolvedTask, root: &std::path::Path) {
/// let options = tak_exec::RunOptions::default();
/// let _future = tak_exec::run_resolved_task(task, root, &options);
/// # }
/// ```
pub async fn run_resolved_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
) -> Result<TaskRunResult> {
    let lease_context = lease_context::LeaseContext::from_options(options);
    let sessions =
        session_workspaces::SharedExecutionSessionManager::new(uuid::Uuid::new_v4().to_string());
    let remote_selection_state = remote_selection::SharedRemoteSelectionState::default();
    run_single_task::run_single_task(run_single_task::RunSingleTaskContext {
        task,
        workspace_root,
        options,
        lease_context: &lease_context,
        sessions: &sessions,
        remote_selection_state: &remote_selection_state,
        execution_label: None,
        placement_override: None,
    })
    .await
}
