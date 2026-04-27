use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

mod attempt_execution;
mod attempt_placement;
mod attempt_submit;
mod lease_context;
mod output_observer;
mod placement;
pub(crate) mod preflight_failure;
pub(crate) mod preflight_fallback;
mod preflight_status_output;
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
#[path = "remote_selection_tests.rs"]
mod remote_selection_tests;
pub(crate) mod remote_submit_failure;
mod remote_wait_status;
mod remote_worker;
mod run_single_task;
mod run_tasks;
mod runtime_metadata;
mod session_cascade;
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

pub use public_types::{
    OutputStream, PlacementMode, RemoteLogChunk, RemoteWorkerExecutionResult,
    RemoteWorkerExecutionSpec, RunOptions, RunSummary, SyncedOutput, TaskOutputChunk,
    TaskOutputObserver, TaskRunResult, TaskStatusEvent, TaskStatusPhase,
};
pub use remote_diagnostics::{
    NoMatchingRemoteError, RemoteCandidateDiagnostic, RemoteCandidateRejection,
    RemotePreflightExhaustedError, RemotePreflightFailure, RemotePreflightFailureKind,
    RequiredRemoteDiagnostic,
};
pub use remote_worker::{execute_remote_worker_steps, execute_remote_worker_steps_with_output};
pub use run_tasks::run_tasks;

pub(crate) use lease_context::LeaseContext;
pub(crate) use output_observer::emit_task_output;
pub(crate) use remote_http_exchange_error::{RemoteHttpExchangeError, RemoteHttpExchangeErrorKind};
pub(crate) use remote_models::{
    ContainerExecutionPlan, ParsedRemoteEvents, RemoteTargetSelection, RemoteWorkspaceStage,
    StrictRemoteTarget,
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
    run_single_task::run_single_task(task, workspace_root, options, &lease_context, None).await
}
