use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use arti_client::TorClient;
use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, LocalSpec,
    PathAnchor, PathRef, PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
    build_current_state_manifest, normalize_path_ref,
};
use tak_loader::evaluate_named_policy_decision;
use tokio::net::TcpStream;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

use crate::client_observations::{load_remote_observation, record_remote_observation};
use crate::client_remotes::configured_remote_targets;
use crate::client_tor::default_client_tor_config;
use crate::container_engine::{
    ContainerEngine, ShellContainerEngineProbe, resolve_container_engine_host_platform,
    select_container_engine_with_probe,
};
use crate::container_runtime::run_task_steps_in_container;
use crate::execution_graph::collect_required_labels;
use crate::lease_client::{acquire_task_lease, release_task_lease};
use crate::remote_endpoint::{remote_protocol_bearer_token, test_tor_onion_dial_addr};
use crate::remote_protocol_codec::{
    build_remote_submit_payload, parse_remote_events_response, parse_remote_result_outputs,
};
use crate::retry::{retry_backoff_delay, should_retry};
use crate::step_runner::{StepRunResult, run_step};

use self::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use self::attempt_submit::{
    preflight_task_placement, resolve_attempt_submit_state, resolve_initial_runtime_metadata,
};
use self::output_observer::emit_task_status_message;
use self::placement::resolve_task_placement;
use self::preflight_failure::{
    RemoteNodeInfoFailure, remote_preflight_error_failure, remote_preflight_timeout_failure,
    remote_preflight_unhealthy_failure,
};
use self::preflight_fallback::{
    fallback_after_auth_submit_failure, preflight_ordered_remote_target,
};
use self::preflight_status_output::{
    emit_remote_accepted, emit_remote_connected, emit_remote_probe, emit_remote_submit,
    emit_remote_unavailable, next_candidate_available,
};
use self::protocol_detection::detect_remote_protocol_mode;
use self::protocol_events::remote_protocol_events;
use self::protocol_result_http::{remote_protocol_result, try_remote_protocol_result};
use self::protocol_submit::remote_protocol_submit;
use self::remote_models::{
    ContainerLifecycleStage, RemoteProtocolResult, RemoteSubmitContext, RuntimeExecutionMetadata,
    TaskPlacement,
};
use self::remote_wait_status::{remote_wait_heartbeat_interval, render_remote_wait_heartbeat};
use self::run_single_task::run_single_task;
use self::runtime_metadata::{
    resolve_runtime_execution_metadata, resolve_runtime_execution_metadata_for_node_runtime,
};
use self::step_execution::run_task_steps_with_runtime;
use self::task_result::build_task_run_result;
use self::transport::TransportFactory;
use self::workspace_collect::{collect_workspace_files, materialize_manifest_files};
use self::workspace_stage::stage_remote_workspace;
use self::workspace_sync::{
    normalize_filesystem_relative_path, sync_remote_outputs, sync_remote_outputs_from_remote,
};

mod attempt_execution;
mod attempt_submit;
mod lease_context;
mod output_observer;
mod placement;
mod preflight_failure;
mod preflight_fallback;
mod preflight_status_output;
mod protocol_detection;
mod protocol_events;
mod protocol_result_http;
mod protocol_submit;
mod public_types;
mod remote_diagnostics;
mod remote_http_exchange_error;
mod remote_models;
mod remote_submit_failure;
mod remote_wait_status;
mod remote_worker;
mod run_single_task;
mod run_tasks;
mod runtime_metadata;
mod step_execution;
mod task_result;
mod transport;
mod transport_tor;
mod workspace_collect;
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
#[allow(unused_imports)]
pub(crate) use preflight_failure::{RemoteNodeInfoFailureKind, classify_preflight_failure_kind};
#[allow(unused_imports)]
pub(crate) use preflight_fallback::is_auth_submit_failure;
#[allow(unused_imports)]
pub(crate) use protocol_result_http::{parse_remote_protocol_result, remote_protocol_http_request};
#[allow(unused_imports)]
pub(crate) use remote_http_exchange_error::{RemoteHttpExchangeError, RemoteHttpExchangeErrorKind};
#[allow(unused_imports)]
pub(crate) use remote_models::RemoteTargetSelection;
#[allow(unused_imports)]
pub(crate) use remote_models::{
    ContainerExecutionPlan, ParsedRemoteEvents, RemoteWorkspaceStage, StrictRemoteTarget,
};
#[allow(unused_imports)]
pub(crate) use remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind};

/// Executes exactly one resolved task and preserves the task's own success and exit metadata.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # async fn demo(
/// #     task: &tak_core::model::ResolvedTask,
/// #     root: &std::path::Path,
/// # ) -> anyhow::Result<()> {
/// let _result = tak_exec::run_resolved_task(task, root, &tak_exec::RunOptions::default()).await?;
/// # Ok(())
/// # }
/// ```
pub async fn run_resolved_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
) -> Result<TaskRunResult> {
    let lease_context = LeaseContext::from_options(options);
    run_single_task(task, workspace_root, options, &lease_context).await
}
