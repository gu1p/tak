//! Task execution engine for resolved workspace tasks.
//!
//! This crate expands target dependencies, enforces execution ordering, applies retry and
//! timeout policy, and optionally coordinates daemon leases around task execution.

use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use arti_client::TorClientConfig;
use base64::Engine;
use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use tak_core::model::{
    PathAnchor, PathRef, PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec,
    RemoteTransportKind, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel,
    WorkspaceSpec, build_current_state_manifest, normalize_path_ref,
};
use tak_loader::evaluate_named_policy_decision;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;
use zip::write::SimpleFileOptions;

mod container_engine;
mod execution_graph;
mod lease_client;
mod remote_events_wait;
mod retry;
mod step_runner;
mod summary;
use container_engine::{
    ContainerEngine, ShellContainerEngineProbe, engine_name, podman_socket_candidates_from_env,
    resolve_container_engine_host_platform, select_container_engine_with_probe,
};
use execution_graph::collect_required_labels;
use lease_client::{acquire_task_lease, release_task_lease};
use remote_events_wait::remote_events_max_wait_duration;
use retry::{retry_backoff_delay, should_retry};
use step_runner::{StepRunResult, resolve_cwd, run_step};
pub use summary::target_set_from_summary;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub jobs: usize,
    pub keep_going: bool,
    pub lease_socket: Option<PathBuf>,
    pub lease_ttl_ms: u64,
    pub lease_poll_interval_ms: u64,
    pub session_id: Option<String>,
    pub user: Option<String>,
}

impl Default for RunOptions {
    /// Returns conservative defaults for local execution and optional lease coordination.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            jobs: 1,
            keep_going: false,
            lease_socket: None,
            lease_ttl_ms: 30_000,
            lease_poll_interval_ms: 200,
            session_id: None,
            user: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskRunResult {
    pub attempts: u32,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub placement_mode: PlacementMode,
    pub remote_node_id: Option<String>,
    pub remote_transport_kind: Option<String>,
    pub decision_reason: Option<String>,
    pub context_manifest_hash: Option<String>,
    pub remote_runtime_kind: Option<String>,
    pub remote_runtime_engine: Option<String>,
    pub remote_logs: Vec<RemoteLogChunk>,
    pub synced_outputs: Vec<SyncedOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLogChunk {
    pub seq: u64,
    pub chunk: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedOutput {
    pub path: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionSpec {
    pub steps: Vec<StepDef>,
    pub timeout_s: Option<u64>,
    pub runtime: Option<RemoteRuntimeSpec>,
    pub node_id: String,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub runtime_kind: Option<String>,
    pub runtime_engine: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementMode {
    Local,
    Remote,
}

impl PlacementMode {
    /// Returns a stable lowercase placement mode marker for CLI/user output.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub results: BTreeMap<TaskLabel, TaskRunResult>,
}

/// Executes targets and their transitive dependencies according to DAG order.
///
/// Each task is run with retry policy and optional lease acquisition around attempts.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_tasks(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
    options: &RunOptions,
) -> Result<RunSummary> {
    if targets.is_empty() {
        bail!("at least one target label is required");
    }
    if options.jobs == 0 {
        bail!("jobs must be >= 1");
    }

    let required = collect_required_labels(spec, targets)?;
    let dep_map: BTreeMap<TaskLabel, Vec<TaskLabel>> = required
        .iter()
        .map(|label| {
            let task = spec
                .tasks
                .get(label)
                .ok_or_else(|| anyhow!("missing task for label {label}"))?;
            Ok((label.clone(), task.deps.clone()))
        })
        .collect::<Result<_>>()?;

    let order = tak_core::planner::topo_sort(&dep_map).context("failed to order task execution")?;
    let mut summary = RunSummary::default();
    let lease_context = LeaseContext::from_options(options);

    for label in order {
        let task = spec
            .tasks
            .get(&label)
            .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;

        let task_result = run_single_task(task, &spec.root, options, &lease_context).await?;
        let failed = !task_result.success;
        summary.results.insert(label.clone(), task_result);

        if failed && !options.keep_going {
            bail!("task {label} failed");
        }
    }

    if summary.results.values().any(|r| !r.success) {
        bail!("one or more tasks failed");
    }

    Ok(summary)
}

#[derive(Debug, Clone)]
pub(crate) struct LeaseContext {
    pub(crate) user: String,
    pub(crate) session_id: String,
}

impl LeaseContext {
    /// Builds a lease context using explicit options or environment-derived defaults.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn from_options(options: &RunOptions) -> Self {
        let user = options.user.clone().unwrap_or_else(|| {
            env::var("USER")
                .or_else(|_| env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string())
        });
        let session_id = options
            .session_id
            .clone()
            .unwrap_or_else(|| format!("tak-{}", Uuid::new_v4()));

        Self { user, session_id }
    }
}

#[derive(Debug, Clone)]
struct StrictRemoteTarget {
    node_id: String,
    endpoint: String,
    transport_kind: RemoteTransportKind,
    service_auth_env: Option<String>,
    runtime: Option<RemoteRuntimeSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteProtocolMode {
    LegacyReachability,
    HandshakeV1 { remote_worker: bool },
}

impl RemoteProtocolMode {
    fn is_handshake_v1(self) -> bool {
        matches!(self, Self::HandshakeV1 { .. })
    }

    fn remote_worker(self) -> bool {
        match self {
            Self::HandshakeV1 { remote_worker } => remote_worker,
            Self::LegacyReachability => false,
        }
    }
}

#[derive(Debug, Clone)]
struct TaskPlacement {
    placement_mode: PlacementMode,
    remote_node_id: Option<String>,
    strict_remote_target: Option<StrictRemoteTarget>,
    ordered_remote_targets: Vec<StrictRemoteTarget>,
    remote_protocol_mode: Option<RemoteProtocolMode>,
    decision_reason: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteProtocolResult {
    success: bool,
    exit_code: Option<i32>,
    synced_outputs: Vec<SyncedOutput>,
    runtime_kind: Option<String>,
    runtime_engine: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemoteSubmitAck {
    remote_worker: bool,
}

#[derive(Debug, Clone)]
struct ParsedRemoteEvents {
    next_seq: u64,
    done: bool,
    remote_logs: Vec<RemoteLogChunk>,
}

#[derive(Debug)]
struct RemoteWorkspaceStage {
    temp_dir: tempfile::TempDir,
    manifest_hash: String,
    archive_zip_base64: String,
}

#[derive(Debug, Clone)]
struct RuntimeExecutionMetadata {
    kind: String,
    engine: Option<String>,
    env_overrides: BTreeMap<String, String>,
    container_plan: Option<ContainerExecutionPlan>,
}

#[derive(Debug, Clone)]
struct ContainerExecutionPlan {
    engine: ContainerEngine,
    image: String,
}

#[derive(Debug, Clone, Copy)]
struct RemoteSubmitContext<'a> {
    task_run_id: &'a str,
    attempt: u32,
    task_label: &'a str,
    remote_workspace: &'a RemoteWorkspaceStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerLifecycleStage {
    Pull,
    Start,
    Runtime,
}

impl ContainerLifecycleStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Start => "start",
            Self::Runtime => "runtime",
        }
    }
}

trait RemoteTransportAdapter {
    #[cfg(test)]
    fn name(&self) -> &'static str;
    fn socket_addr(&self, endpoint: &str) -> Result<String>;
    fn preflight_timeout(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn min_phase_timeout(&self) -> Duration {
        Duration::ZERO
    }
    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>>;
}

struct DirectHttpsTransportAdapter;
struct TorTransportAdapter;
trait RemoteIo: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
type RemoteIoStream = Box<dyn RemoteIo + Unpin + Send>;

impl RemoteTransportAdapter for DirectHttpsTransportAdapter {
    #[cfg(test)]
    fn name(&self) -> &'static str {
        "direct"
    }

    fn socket_addr(&self, endpoint: &str) -> Result<String> {
        endpoint_socket_addr(endpoint)
    }

    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>> {
        Box::pin(async move {
            let socket_addr = self.socket_addr(&target.endpoint)?;
            let stream = TcpStream::connect(&socket_addr).await?;
            let stream: RemoteIoStream = Box::new(stream);
            Ok(stream)
        })
    }
}

impl RemoteTransportAdapter for TorTransportAdapter {
    #[cfg(test)]
    fn name(&self) -> &'static str {
        "tor"
    }

    fn socket_addr(&self, endpoint: &str) -> Result<String> {
        endpoint_socket_addr(endpoint)
    }

    fn preflight_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn min_phase_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>> {
        Box::pin(async move {
            let (host, port) = endpoint_host_port(&target.endpoint)?;
            if !host.ends_with(".onion") {
                let socket_addr = format!("{host}:{port}");
                let stream = TcpStream::connect(&socket_addr).await?;
                let stream: RemoteIoStream = Box::new(stream);
                return Ok(stream);
            }

            if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
                let stream = TcpStream::connect(&test_dial_addr).await.with_context(|| {
                    format!(
                        "infra error: remote node {} unavailable at {}",
                        target.node_id, target.endpoint
                    )
                })?;
                let stream: RemoteIoStream = Box::new(stream);
                return Ok(stream);
            }

            let tor_client =
                arti_client::TorClient::create_bootstrapped(TorClientConfig::default())
                    .await
                    .with_context(|| {
                        format!(
                            "infra error: remote node {} unavailable at {}",
                            target.node_id, target.endpoint
                        )
                    })?;
            let stream = tor_client
                .connect((host.as_str(), port))
                .await
                .with_context(|| {
                    format!(
                        "infra error: remote node {} unavailable at {}",
                        target.node_id, target.endpoint
                    )
                })?;
            let stream: RemoteIoStream = Box::new(stream);
            Ok(stream)
        })
    }
}

static DIRECT_HTTPS_TRANSPORT_ADAPTER: DirectHttpsTransportAdapter = DirectHttpsTransportAdapter;
static TOR_TRANSPORT_ADAPTER: TorTransportAdapter = TorTransportAdapter;

struct TransportFactory;

impl TransportFactory {
    fn adapter(kind: RemoteTransportKind) -> &'static dyn RemoteTransportAdapter {
        match kind {
            RemoteTransportKind::DirectHttps => &DIRECT_HTTPS_TRANSPORT_ADAPTER,
            RemoteTransportKind::Tor => &TOR_TRANSPORT_ADAPTER,
        }
    }

    #[cfg(test)]
    fn transport_name(kind: RemoteTransportKind) -> &'static str {
        Self::adapter(kind).name()
    }

    fn socket_addr(target: &StrictRemoteTarget) -> Result<String> {
        Self::adapter(target.transport_kind).socket_addr(&target.endpoint)
    }

    fn connect(
        target: &StrictRemoteTarget,
    ) -> impl Future<Output = Result<RemoteIoStream>> + Send + '_ {
        Self::adapter(target.transport_kind).connect_stream(target)
    }

    fn preflight_timeout(target: &StrictRemoteTarget) -> Duration {
        Self::adapter(target.transport_kind).preflight_timeout()
    }

    fn phase_timeout(target: &StrictRemoteTarget, requested: Duration) -> Duration {
        requested.max(Self::adapter(target.transport_kind).min_phase_timeout())
    }
}

/// Runs one task with retries, acquiring and releasing leases per attempt when configured.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn run_single_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
) -> Result<TaskRunResult> {
    let mut placement = resolve_task_placement(task, workspace_root)?;
    if let Some(target) = &placement.strict_remote_target {
        let mode = preflight_strict_remote_target(target).await?;
        if should_reject_legacy_remote_mode(task, target, mode) {
            bail!("{}", legacy_protocol_error_message(target));
        }
        placement.remote_protocol_mode = Some(mode);
    } else if !placement.ordered_remote_targets.is_empty() {
        let (selected, mode) =
            preflight_ordered_remote_target(task, &placement.ordered_remote_targets).await?;
        placement.remote_node_id = Some(selected.node_id.clone());
        placement.strict_remote_target = Some(selected);
        placement.remote_protocol_mode = Some(mode);
    }
    let initial_protocol_mode = placement
        .remote_protocol_mode
        .unwrap_or(RemoteProtocolMode::LegacyReachability);
    let mut runtime_metadata = if placement.placement_mode == PlacementMode::Remote
        && initial_protocol_mode.remote_worker()
    {
        None
    } else {
        match resolve_runtime_execution_metadata(task, &placement) {
            Ok(metadata) => metadata,
            Err(runtime_error) => {
                if placement.ordered_remote_targets.is_empty()
                    || !is_container_lifecycle_failure(&runtime_error)
                {
                    return Err(runtime_error);
                }

                let failed_node_id = placement
                    .strict_remote_target
                    .as_ref()
                    .map(|target| target.node_id.clone())
                    .ok_or_else(|| {
                        anyhow!(
                            "infra error: missing strict remote target during runtime metadata resolution for task {}",
                            task.label
                        )
                    })?;
                let (fallback_target, fallback_mode, fallback_runtime_metadata) =
                    fallback_after_container_lifecycle_failure(
                        task,
                        &placement.ordered_remote_targets,
                        &failed_node_id,
                        runtime_error.to_string(),
                    )
                    .await?;
                placement.remote_node_id = Some(fallback_target.node_id.clone());
                placement.strict_remote_target = Some(fallback_target);
                placement.remote_protocol_mode = Some(fallback_mode);
                fallback_runtime_metadata
            }
        }
    };
    let remote_workspace = if placement.placement_mode == PlacementMode::Remote {
        Some(stage_remote_workspace(task, workspace_root)?)
    } else {
        None
    };
    let run_root = remote_workspace
        .as_ref()
        .map(|staged| staged.temp_dir.path())
        .unwrap_or(workspace_root);

    let total_attempts = task.retry.attempts.max(1);
    let mut attempt = 0;
    let task_run_id = Uuid::new_v4().to_string();
    let task_label = task.label.to_string();

    loop {
        attempt += 1;

        let mut protocol_mode = placement
            .remote_protocol_mode
            .unwrap_or(RemoteProtocolMode::LegacyReachability);
        let mut submit_ack = RemoteSubmitAck {
            remote_worker: false,
        };
        if placement.placement_mode == PlacementMode::Remote && protocol_mode.is_handshake_v1() {
            let target = placement.strict_remote_target.clone().ok_or_else(|| {
                anyhow!(
                    "infra error: missing strict remote target during submit for task {}",
                    task.label
                )
            })?;
            let remote_workspace = remote_workspace.as_ref().ok_or_else(|| {
                anyhow!(
                    "infra error: missing staged remote workspace during submit for task {}",
                    task.label
                )
            })?;
            match remote_protocol_submit(
                &target,
                &task_run_id,
                attempt,
                &task_label,
                task,
                remote_workspace,
                protocol_mode.remote_worker(),
            )
            .await
            {
                Ok(ack) => {
                    submit_ack = ack;
                }
                Err(submit_error) => {
                    if !placement.ordered_remote_targets.is_empty()
                        && is_auth_submit_failure(&submit_error)
                    {
                        let failed_node_id = target.node_id.clone();
                        let (fallback_target, fallback_mode, fallback_ack) =
                            fallback_after_auth_submit_failure(
                                task,
                                &placement.ordered_remote_targets,
                                &failed_node_id,
                                RemoteSubmitContext {
                                    task_run_id: &task_run_id,
                                    attempt,
                                    task_label: &task_label,
                                    remote_workspace,
                                },
                                submit_error.to_string(),
                            )
                            .await?;
                        placement.remote_node_id = Some(fallback_target.node_id.clone());
                        placement.strict_remote_target = Some(fallback_target);
                        placement.remote_protocol_mode = Some(fallback_mode);
                        protocol_mode = fallback_mode;
                        if !protocol_mode.remote_worker() {
                            runtime_metadata =
                                resolve_runtime_execution_metadata(task, &placement)?;
                        }
                        submit_ack = fallback_ack;
                    } else {
                        return Err(submit_error);
                    }
                }
            }
        }

        let lease_id = acquire_task_lease(task, attempt, options, lease_context).await?;
        let delegate_v1_remote_attempt = placement.placement_mode == PlacementMode::Remote
            && protocol_mode.is_handshake_v1()
            && (submit_ack.remote_worker || runtime_metadata.is_none());
        let run_local_attempt = !delegate_v1_remote_attempt;
        let run_result = if run_local_attempt {
            run_task_steps_with_runtime(task, run_root, runtime_metadata.as_ref()).await
        } else {
            Ok(StepRunResult {
                success: true,
                exit_code: Some(0),
            })
        };

        let (remote_logs, protocol_result) = if placement.placement_mode == PlacementMode::Remote
            && protocol_mode.is_handshake_v1()
        {
            let target = placement.strict_remote_target.as_ref().ok_or_else(|| {
                anyhow!(
                    "infra error: missing strict remote target during events/result for task {}",
                    task.label
                )
            })?;
            let remote_logs = remote_protocol_events(target, &task_run_id).await?;
            let result = remote_protocol_result(target, &task_run_id, attempt).await?;
            (remote_logs, Some(result))
        } else {
            (Vec::new(), None)
        };

        let release_result = match &lease_id {
            Some(id) => release_task_lease(id, options).await,
            None => Ok(()),
        };

        if let Err(err) = release_result {
            return Err(err).context(format!("failed releasing lease for {}", task.label));
        }

        let run = run_result?;
        let (
            attempt_success,
            last_exit_code,
            synced_outputs,
            remote_runtime_kind,
            remote_runtime_engine,
        ) = match protocol_result {
            Some(remote_result) => (
                remote_result.success,
                remote_result.exit_code.or(run.exit_code),
                remote_result.synced_outputs,
                remote_result.runtime_kind,
                remote_result.runtime_engine,
            ),
            None => (run.success, run.exit_code, Vec::new(), None, None),
        };
        if !synced_outputs.is_empty() {
            if run_local_attempt {
                if let Some(staged_workspace) = remote_workspace.as_ref() {
                    sync_remote_outputs(
                        staged_workspace.temp_dir.path(),
                        workspace_root,
                        &synced_outputs,
                    )?;
                }
            } else if submit_ack.remote_worker {
                let target = placement.strict_remote_target.as_ref().ok_or_else(|| {
                    anyhow!(
                        "infra error: missing strict remote target during output sync for task {}",
                        task.label
                    )
                })?;
                sync_remote_outputs_from_remote(
                    target,
                    &task_run_id,
                    attempt,
                    workspace_root,
                    &synced_outputs,
                )
                .await?;
            }
        }

        if attempt_success {
            return Ok(TaskRunResult {
                attempts: attempt,
                success: true,
                exit_code: last_exit_code,
                placement_mode: placement.placement_mode,
                remote_node_id: placement.remote_node_id.clone(),
                remote_transport_kind: placement
                    .strict_remote_target
                    .as_ref()
                    .map(|target| target.transport_kind.as_result_value().to_string()),
                decision_reason: placement.decision_reason.clone(),
                context_manifest_hash: remote_workspace
                    .as_ref()
                    .map(|staged| staged.manifest_hash.clone()),
                remote_runtime_kind: remote_runtime_kind
                    .or_else(|| runtime_metadata.as_ref().map(|meta| meta.kind.clone())),
                remote_runtime_engine: remote_runtime_engine.or_else(|| {
                    runtime_metadata
                        .as_ref()
                        .and_then(|meta| meta.engine.clone())
                }),
                remote_logs,
                synced_outputs,
            });
        }

        let can_retry =
            attempt < total_attempts && should_retry(last_exit_code, &task.retry.on_exit);
        if !can_retry {
            return Ok(TaskRunResult {
                attempts: attempt,
                success: false,
                exit_code: last_exit_code,
                placement_mode: placement.placement_mode,
                remote_node_id: placement.remote_node_id.clone(),
                remote_transport_kind: placement
                    .strict_remote_target
                    .as_ref()
                    .map(|target| target.transport_kind.as_result_value().to_string()),
                decision_reason: placement.decision_reason.clone(),
                context_manifest_hash: remote_workspace
                    .as_ref()
                    .map(|staged| staged.manifest_hash.clone()),
                remote_runtime_kind: remote_runtime_kind
                    .or_else(|| runtime_metadata.as_ref().map(|meta| meta.kind.clone())),
                remote_runtime_engine: remote_runtime_engine.or_else(|| {
                    runtime_metadata
                        .as_ref()
                        .and_then(|meta| meta.engine.clone())
                }),
                remote_logs,
                synced_outputs,
            });
        }

        let wait = retry_backoff_delay(&task.retry.backoff, attempt);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// Creates a staged workspace for remote execution from the task's normalized `CurrentState`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn stage_remote_workspace(
    task: &ResolvedTask,
    workspace_root: &Path,
) -> Result<RemoteWorkspaceStage> {
    let available_files = collect_workspace_files(workspace_root)?;
    let manifest = build_current_state_manifest(available_files, &task.context);
    let staged_dir = tempfile::tempdir().context("failed to create staged remote workspace")?;
    materialize_manifest_files(workspace_root, staged_dir.path(), &manifest.entries)?;
    let archive_zip_base64 = build_zip_snapshot_base64(staged_dir.path())?;

    Ok(RemoteWorkspaceStage {
        temp_dir: staged_dir,
        manifest_hash: manifest.hash,
        archive_zip_base64,
    })
}

fn build_zip_snapshot_base64(staged_root: &Path) -> Result<String> {
    let mut archive_bytes = Vec::<u8>::new();
    {
        let cursor = std::io::Cursor::new(&mut archive_bytes);
        let mut zip = zip::ZipWriter::new(cursor);
        write_zip_entries_recursive(staged_root, staged_root, &mut zip)?;
        zip.finish()
            .context("failed finishing staged workspace zip snapshot")?;
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(archive_bytes))
}

fn write_zip_entries_recursive<W: Write + std::io::Seek>(
    staged_root: &Path,
    current_dir: &Path,
    zip: &mut zip::ZipWriter<W>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read staged directory {}", current_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!(
                "failed to read staged entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read staged file type for {}",
                path.to_string_lossy()
            )
        })?;
        if file_type.is_dir() {
            write_zip_entries_recursive(staged_root, &path, zip)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path.strip_prefix(staged_root).with_context(|| {
            format!(
                "failed to compute staged relative path {} under {}",
                path.display(),
                staged_root.display()
            )
        })?;
        let name = normalize_filesystem_relative_path(relative);
        let options = SimpleFileOptions::default();
        zip.start_file(&name, options)
            .with_context(|| format!("failed to start zip entry {name}"))?;
        let mut file = fs::File::open(&path)
            .with_context(|| format!("failed to open staged file {}", path.display()))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("failed to read staged file {}", path.display()))?;
        zip.write_all(&buffer)
            .with_context(|| format!("failed to write zip entry {name}"))?;
    }

    Ok(())
}

/// Collects all regular files under the workspace root as normalized workspace-anchored refs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn collect_workspace_files(workspace_root: &Path) -> Result<Vec<PathRef>> {
    let mut files = Vec::new();
    collect_workspace_files_recursive(workspace_root, workspace_root, &mut files)?;
    Ok(files)
}

fn collect_workspace_files_recursive(
    workspace_root: &Path,
    current_dir: &Path,
    files: &mut Vec<PathRef>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir).with_context(|| {
        format!(
            "failed to read workspace directory {}",
            current_dir.display()
        )
    })? {
        let entry = entry.with_context(|| {
            format!(
                "failed to read workspace entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read file type for workspace entry {}",
                path.display()
            )
        })?;

        if file_type.is_dir() {
            collect_workspace_files_recursive(workspace_root, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path.strip_prefix(workspace_root).with_context(|| {
            format!(
                "failed to compute relative path for workspace file {}",
                path.display()
            )
        })?;
        files.push(PathRef {
            anchor: PathAnchor::Workspace,
            path: normalize_filesystem_relative_path(relative),
        });
    }

    Ok(())
}

/// Copies manifest-selected files into the staged workspace while preserving relative layout.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn materialize_manifest_files(
    workspace_root: &Path,
    staged_root: &Path,
    entries: &[PathRef],
) -> Result<()> {
    for entry in entries {
        if entry.anchor != PathAnchor::Workspace {
            bail!(
                "unsupported non-workspace context manifest anchor during staging: {:?}",
                entry.anchor
            );
        }
        if entry.path == "." {
            continue;
        }

        let source = workspace_root.join(&entry.path);
        if !source.is_file() {
            continue;
        }
        let destination = staged_root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create staged directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to stage context file {} -> {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

fn sync_remote_outputs(
    staged_root: &Path,
    workspace_root: &Path,
    outputs: &[SyncedOutput],
) -> Result<()> {
    for output in outputs {
        let relative_path = normalized_synced_output_path(output)?;
        let source = staged_root.join(&relative_path);
        if !source.is_file() {
            continue;
        }

        let destination = workspace_root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create output sync directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to sync remote output {} -> {}",
                source.display(),
                destination.display()
            )
        })?;

        let copied_size = fs::metadata(&destination)
            .with_context(|| format!("failed to stat synced output {}", destination.display()))?
            .len();
        if copied_size != output.size_bytes {
            bail!(
                "infra error: remote output {} size mismatch after sync (expected {}, got {})",
                output.path,
                output.size_bytes,
                copied_size
            );
        }
    }

    Ok(())
}

async fn sync_remote_outputs_from_remote(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace_root: &Path,
    outputs: &[SyncedOutput],
) -> Result<()> {
    for output in outputs {
        let relative_path = normalized_synced_output_path(output)?;
        let path_query = relative_path.to_string_lossy().to_string();
        let request_path =
            format!("/v1/tasks/{task_run_id}/outputs?attempt={attempt}&path={path_query}");
        let (status, response_body) = remote_protocol_http_request(
            target,
            "GET",
            &request_path,
            None,
            "outputs",
            Duration::from_secs(2),
        )
        .await?;
        if status != 200 {
            bail!(
                "infra error: remote node {} output download failed for {} with HTTP {}",
                target.node_id,
                output.path,
                status
            );
        }

        let parsed: serde_json::Value =
            serde_json::from_str(&response_body).with_context(|| {
                format!(
                    "infra error: remote node {} output download payload is invalid JSON for {}",
                    target.node_id, output.path
                )
            })?;
        let encoded = parsed
            .get("data_base64")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} output download payload is missing data_base64 for {}",
                    target.node_id,
                    output.path
                )
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .with_context(|| {
                format!(
                    "infra error: remote node {} output download payload has invalid base64 for {}",
                    target.node_id, output.path
                )
            })?;

        let destination = workspace_root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create output sync directory {}",
                    parent.to_string_lossy()
                )
            })?;
        }
        fs::write(&destination, &bytes).with_context(|| {
            format!(
                "failed to write remote output {} to {}",
                output.path,
                destination.display()
            )
        })?;

        let copied_size = u64::try_from(bytes.len()).unwrap_or(0);
        if copied_size != output.size_bytes {
            bail!(
                "infra error: remote output {} size mismatch after download (expected {}, got {})",
                output.path,
                output.size_bytes,
                copied_size
            );
        }

        let expected_digest = output
            .digest
            .strip_prefix("sha256:")
            .unwrap_or(output.digest.as_str())
            .to_string();
        let actual_digest = format!("{:x}", Sha256::digest(&bytes));
        if actual_digest != expected_digest {
            bail!(
                "infra error: remote output {} digest mismatch after download",
                output.path
            );
        }
    }

    Ok(())
}

fn normalized_synced_output_path(output: &SyncedOutput) -> Result<PathBuf> {
    let normalized = normalize_path_ref("workspace", &output.path).map_err(|err| {
        anyhow!(
            "infra error: remote output path `{}` is invalid: {err}",
            output.path
        )
    })?;
    if normalized.path == "." {
        bail!(
            "infra error: remote output path `{}` must reference a file",
            output.path
        );
    }
    Ok(PathBuf::from(normalized.path))
}

fn normalize_filesystem_relative_path(path: &Path) -> String {
    let mut value = String::new();
    for component in path.components() {
        if !value.is_empty() {
            value.push('/');
        }
        value.push_str(&component.as_os_str().to_string_lossy());
    }
    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}

fn resolve_runtime_execution_metadata(
    task: &ResolvedTask,
    placement: &TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(None);
    }

    let Some(target) = placement.strict_remote_target.as_ref() else {
        return Ok(None);
    };
    resolve_runtime_execution_metadata_for_target(task, target)
}

fn resolve_runtime_execution_metadata_for_target(
    task: &ResolvedTask,
    target: &StrictRemoteTarget,
) -> Result<Option<RuntimeExecutionMetadata>> {
    resolve_runtime_execution_metadata_for_node_runtime(
        task,
        &target.node_id,
        target.runtime.as_ref(),
    )
}

fn resolve_runtime_execution_metadata_for_node_runtime(
    task: &ResolvedTask,
    node_id: &str,
    runtime: Option<&RemoteRuntimeSpec>,
) -> Result<Option<RuntimeExecutionMetadata>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };

    match runtime {
        RemoteRuntimeSpec::Containerized { image } => {
            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Pull,
            )?;

            let mut probe = ShellContainerEngineProbe;
            let engine = select_container_engine_with_probe(
                resolve_container_engine_host_platform(),
                &mut probe,
            )
            .map_err(|err| {
                anyhow!(
                    "infra error: remote node {} container lifecycle {} failed for task {}: {}",
                    node_id,
                    ContainerLifecycleStage::Start.as_str(),
                    task.label,
                    err
                )
            })?;

            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Start,
            )?;

            let engine_name = match engine {
                ContainerEngine::Docker => "docker".to_string(),
                ContainerEngine::Podman => "podman".to_string(),
            };

            let mut env_overrides = BTreeMap::new();
            env_overrides.insert(
                "TAK_REMOTE_RUNTIME".to_string(),
                "containerized".to_string(),
            );
            env_overrides.insert("TAK_REMOTE_ENGINE".to_string(), engine_name.clone());
            env_overrides.insert("TAK_REMOTE_CONTAINER_IMAGE".to_string(), image.clone());

            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Runtime,
            )?;

            let container_plan = if should_use_simulated_container_runtime() {
                None
            } else {
                Some(ContainerExecutionPlan {
                    engine,
                    image: image.clone(),
                })
            };

            Ok(Some(RuntimeExecutionMetadata {
                kind: "containerized".to_string(),
                engine: Some(engine_name),
                env_overrides,
                container_plan,
            }))
        }
    }
}

fn should_use_simulated_container_runtime() -> bool {
    env::var("TAK_TEST_HOST_PLATFORM").is_ok()
        || env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_ok()
}

fn maybe_fail_injected_container_lifecycle_stage(
    task: &ResolvedTask,
    node_id: &str,
    stage: ContainerLifecycleStage,
) -> Result<()> {
    let Some(injected_stage) = test_injected_container_lifecycle_stage(node_id) else {
        return Ok(());
    };
    if injected_stage != stage {
        return Ok(());
    }

    bail!(
        "infra error: remote node {} container lifecycle {} failed for task {}: simulated deterministic failure",
        node_id,
        stage.as_str(),
        task.label
    );
}

fn test_injected_container_lifecycle_stage(node_id: &str) -> Option<ContainerLifecycleStage> {
    let configured = env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").ok()?;
    for entry in configured.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some((entry_node, raw_stage)) = entry.split_once(':') else {
            continue;
        };
        if entry_node.trim() != node_id {
            continue;
        }

        let stage = raw_stage
            .trim()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        return match stage.as_str() {
            "pull" => Some(ContainerLifecycleStage::Pull),
            "start" => Some(ContainerLifecycleStage::Start),
            "runtime" => Some(ContainerLifecycleStage::Runtime),
            _ => None,
        };
    }

    None
}

/// Resolves the execution constructor into current placement metadata and validates support.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_task_placement(task: &ResolvedTask, workspace_root: &Path) -> Result<TaskPlacement> {
    match &task.execution {
        TaskExecutionSpec::LocalOnly(local) => {
            // Local constructor metadata is validated by the loader and preserved for summaries.
            let _ = local.max_parallel_tasks;
            let _ = &local.id;
            Ok(TaskPlacement {
                placement_mode: PlacementMode::Local,
                remote_node_id: None,
                strict_remote_target: None,
                ordered_remote_targets: Vec::new(),
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(remote)) => {
            let endpoint = remote_endpoint_for_strict(
                remote,
                "strict pin execution",
                &task.label.to_string(),
            )?;
            Ok(TaskPlacement {
                placement_mode: PlacementMode::Remote,
                remote_node_id: Some(remote.id.clone()),
                strict_remote_target: Some(StrictRemoteTarget {
                    node_id: remote.id.clone(),
                    endpoint,
                    transport_kind: remote.transport_kind,
                    service_auth_env: remote.service_auth_env.clone(),
                    runtime: remote.runtime.clone(),
                }),
                ordered_remote_targets: Vec::new(),
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(remotes)) => {
            if remotes.is_empty() {
                bail!(
                    "infra error: task {} has no remote fallback candidates",
                    task.label
                );
            }

            let mut ordered_remote_targets = Vec::with_capacity(remotes.len());
            for remote in remotes {
                let endpoint = remote_endpoint_for_strict(
                    remote,
                    "fallback execution",
                    &task.label.to_string(),
                )?;
                ordered_remote_targets.push(StrictRemoteTarget {
                    node_id: remote.id.clone(),
                    endpoint,
                    transport_kind: remote.transport_kind,
                    service_auth_env: remote.service_auth_env.clone(),
                    runtime: remote.runtime.clone(),
                });
            }

            Ok(TaskPlacement {
                placement_mode: PlacementMode::Remote,
                remote_node_id: None,
                strict_remote_target: None,
                ordered_remote_targets,
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let resolved_decision = if let Some(decision) = decision.as_ref() {
                decision.clone()
            } else {
                let tasks_file = tasks_file_for_label(workspace_root, &task.label);
                evaluate_named_policy_decision(&tasks_file, policy_name).with_context(|| {
                    format!(
                        "runtime policy evaluation failed for task {} (policy={policy_name})",
                        task.label
                    )
                })?
            };
            match &resolved_decision {
                PolicyDecisionSpec::Local { reason } => Ok(TaskPlacement {
                    placement_mode: PlacementMode::Local,
                    remote_node_id: None,
                    strict_remote_target: None,
                    ordered_remote_targets: Vec::new(),
                    remote_protocol_mode: None,
                    decision_reason: Some(reason.clone()),
                }),
                PolicyDecisionSpec::Remote { reason, remote } => {
                    let endpoint = remote_endpoint_for_strict(
                        remote,
                        "policy strict remote execution",
                        &task.label.to_string(),
                    )?;
                    Ok(TaskPlacement {
                        placement_mode: PlacementMode::Remote,
                        remote_node_id: Some(remote.id.clone()),
                        strict_remote_target: Some(StrictRemoteTarget {
                            node_id: remote.id.clone(),
                            endpoint,
                            transport_kind: remote.transport_kind,
                            service_auth_env: remote.service_auth_env.clone(),
                            runtime: remote.runtime.clone(),
                        }),
                        ordered_remote_targets: Vec::new(),
                        remote_protocol_mode: None,
                        decision_reason: Some(reason.clone()),
                    })
                }
                PolicyDecisionSpec::RemoteAny { reason, remotes } => {
                    if remotes.is_empty() {
                        bail!(
                            "infra error: policy decision for task {} has no remote fallback candidates",
                            task.label
                        );
                    }

                    let mut ordered_remote_targets = Vec::with_capacity(remotes.len());
                    for remote in remotes {
                        let endpoint = remote_endpoint_for_strict(
                            remote,
                            "policy fallback execution",
                            &task.label.to_string(),
                        )?;
                        ordered_remote_targets.push(StrictRemoteTarget {
                            node_id: remote.id.clone(),
                            endpoint,
                            transport_kind: remote.transport_kind,
                            service_auth_env: remote.service_auth_env.clone(),
                            runtime: remote.runtime.clone(),
                        });
                    }

                    Ok(TaskPlacement {
                        placement_mode: PlacementMode::Remote,
                        remote_node_id: None,
                        strict_remote_target: None,
                        ordered_remote_targets,
                        remote_protocol_mode: None,
                        decision_reason: Some(reason.clone()),
                    })
                }
            }
        }
    }
}

fn tasks_file_for_label(workspace_root: &Path, label: &TaskLabel) -> PathBuf {
    if label.package == "//" {
        return workspace_root.join("TASKS.py");
    }

    let package = label.package.trim_start_matches("//");
    workspace_root.join(package).join("TASKS.py")
}

/// Resolves a strict remote endpoint value or returns a contextual infra error.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn remote_endpoint_for_strict(remote: &RemoteSpec, mode: &str, task_label: &str) -> Result<String> {
    remote.endpoint.clone().ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} is missing endpoint for {mode} in task {task_label}",
            remote.id
        )
    })
}

/// Selects the first reachable remote endpoint in declaration order.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn preflight_ordered_remote_target(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
) -> Result<(StrictRemoteTarget, RemoteProtocolMode)> {
    let mut failures = Vec::new();

    for candidate in candidates {
        match preflight_strict_remote_target(candidate).await {
            Ok(mode) => {
                if should_reject_legacy_remote_mode(task, candidate, mode) {
                    failures.push(legacy_protocol_error_message(candidate));
                    continue;
                }
                return Ok((candidate.clone(), mode));
            }
            Err(err) => failures.push(err.to_string()),
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

fn should_reject_legacy_remote_mode(
    task: &ResolvedTask,
    target: &StrictRemoteTarget,
    mode: RemoteProtocolMode,
) -> bool {
    matches!(mode, RemoteProtocolMode::LegacyReachability)
        && matches!(task.execution, TaskExecutionSpec::RemoteOnly(_))
        && target.runtime.is_none()
}

fn legacy_protocol_error_message(target: &StrictRemoteTarget) -> String {
    format!(
        "infra error: remote node {} at {} does not support V1 handshake protocol",
        target.node_id, target.endpoint
    )
}

/// Performs strict remote preflight by checking endpoint reachability before task execution.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn preflight_strict_remote_target(target: &StrictRemoteTarget) -> Result<RemoteProtocolMode> {
    TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;

    let preflight_timeout = TransportFactory::preflight_timeout(target);
    match tokio::time::timeout(preflight_timeout, TransportFactory::connect(target)).await {
        Ok(Ok(stream)) => {
            drop(stream);
            detect_remote_protocol_mode(target).await
        }
        Ok(Err(err)) => bail!(
            "infra error: remote node {} unavailable at {}: {err}",
            target.node_id,
            target.endpoint
        ),
        Err(_) => bail!(
            "infra error: remote node {} unavailable at {}: preflight timed out",
            target.node_id,
            target.endpoint
        ),
    }
}

fn is_auth_submit_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("auth failed")
}

fn is_auth_configuration_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("service auth token")
}

fn is_container_lifecycle_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("container lifecycle")
}

async fn fallback_after_container_lifecycle_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    initial_failure: String,
) -> Result<(
    StrictRemoteTarget,
    RemoteProtocolMode,
    Option<RuntimeExecutionMetadata>,
)> {
    let mut failures = vec![initial_failure];

    for candidate in candidates {
        if candidate.node_id == failed_node_id {
            continue;
        }

        let mode = match preflight_strict_remote_target(candidate).await {
            Ok(mode) => mode,
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        };

        match resolve_runtime_execution_metadata_for_target(task, candidate) {
            Ok(runtime_metadata) => return Ok((candidate.clone(), mode, runtime_metadata)),
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

async fn fallback_after_auth_submit_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    submit: RemoteSubmitContext<'_>,
    initial_failure: String,
) -> Result<(StrictRemoteTarget, RemoteProtocolMode, RemoteSubmitAck)> {
    let mut failures = vec![initial_failure];

    for candidate in candidates {
        if candidate.node_id == failed_node_id {
            continue;
        }

        let mode = match preflight_strict_remote_target(candidate).await {
            Ok(mode) => mode,
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        };

        if mode.is_handshake_v1() {
            match remote_protocol_submit(
                candidate,
                submit.task_run_id,
                submit.attempt,
                submit.task_label,
                task,
                submit.remote_workspace,
                mode.remote_worker(),
            )
            .await
            {
                Ok(ack) => return Ok((candidate.clone(), mode, ack)),
                Err(err) => {
                    failures.push(err.to_string());
                    continue;
                }
            }
        } else {
            return Ok((
                candidate.clone(),
                mode,
                RemoteSubmitAck {
                    remote_worker: false,
                },
            ));
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

/// Probes whether the remote endpoint supports the V1 handshake preflight contract.
///
/// Unsupported or legacy endpoints silently degrade to reachability-only behavior.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn detect_remote_protocol_mode(target: &StrictRemoteTarget) -> Result<RemoteProtocolMode> {
    let capabilities = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/capabilities",
        None,
        "capabilities",
        Duration::from_millis(150),
    )
    .await;

    let (status, body) = match capabilities {
        Ok(response) => response,
        Err(err) => {
            if is_auth_configuration_failure(&err) {
                return Err(err);
            }
            return Ok(RemoteProtocolMode::LegacyReachability);
        }
    };

    if status == 401 || status == 403 {
        bail!(
            "infra error: remote node {} auth failed during capabilities with HTTP {}",
            target.node_id,
            status
        );
    }
    if status != 200 {
        return Ok(RemoteProtocolMode::LegacyReachability);
    }

    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(value) => value,
        Err(_) => return Ok(RemoteProtocolMode::LegacyReachability),
    };
    let Some(compatible) = parsed
        .get("compatible")
        .and_then(serde_json::Value::as_bool)
    else {
        return Ok(RemoteProtocolMode::LegacyReachability);
    };

    if !compatible {
        bail!(
            "infra error: remote node {} capability mismatch at {}",
            target.node_id,
            target.endpoint
        );
    }

    let status_response = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/status",
        None,
        "status",
        Duration::from_millis(150),
    )
    .await;
    let (status_code, status_body) = match status_response {
        Ok(response) => response,
        Err(err) => {
            if is_auth_configuration_failure(&err) {
                return Err(err);
            }
            return Ok(RemoteProtocolMode::LegacyReachability);
        }
    };
    if status_code == 401 || status_code == 403 {
        bail!(
            "infra error: remote node {} auth failed during status with HTTP {}",
            target.node_id,
            status_code
        );
    }
    if status_code != 200 {
        return Ok(RemoteProtocolMode::LegacyReachability);
    }
    if let Ok(parsed_status) = serde_json::from_str::<serde_json::Value>(&status_body)
        && let Some(healthy) = parsed_status
            .get("healthy")
            .and_then(serde_json::Value::as_bool)
        && !healthy
    {
        bail!(
            "infra error: remote node {} reported unhealthy status at {}",
            target.node_id,
            target.endpoint
        );
    }

    let remote_worker = parsed
        .get("remote_worker")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            parsed
                .get("execution_mode")
                .and_then(serde_json::Value::as_str)
                .map(|mode| mode == "remote_worker")
        })
        .unwrap_or(false);

    Ok(RemoteProtocolMode::HandshakeV1 { remote_worker })
}

/// Submits one remote attempt after successful preflight.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
    include_workspace_archive: bool,
) -> Result<RemoteSubmitAck> {
    let body = build_remote_submit_payload(
        target,
        task_run_id,
        attempt,
        task_label,
        task,
        remote_workspace,
        include_workspace_archive,
    )?
    .to_string();

    let (status, response_body) = remote_protocol_http_request(
        target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        Duration::from_secs(1),
    )
    .await?;

    if status == 401 || status == 403 {
        bail!(
            "infra error: remote node {} auth failed during submit with HTTP {}",
            target.node_id,
            status
        );
    }
    if status != 200 {
        bail!(
            "infra error: remote node {} submit failed with HTTP {}",
            target.node_id,
            status
        );
    }

    let parsed = serde_json::from_str::<serde_json::Value>(&response_body).ok();
    let accepted = parsed
        .as_ref()
        .and_then(|value| value.get("accepted").and_then(serde_json::Value::as_bool))
        .unwrap_or(true);
    if !accepted {
        let is_auth_rejection = parsed
            .as_ref()
            .and_then(|value| value.get("reason").and_then(serde_json::Value::as_str))
            .map(|reason| reason.eq_ignore_ascii_case("auth_failed"))
            .unwrap_or(false);
        if is_auth_rejection {
            bail!(
                "infra error: remote node {} auth failed during submit",
                target.node_id
            );
        }
        bail!(
            "infra error: remote node {} rejected submit for task {} attempt {}",
            target.node_id,
            task_label,
            attempt
        );
    }

    let remote_worker = parsed
        .as_ref()
        .and_then(|value| {
            value
                .get("execution_mode")
                .and_then(serde_json::Value::as_str)
        })
        .map(|mode| mode == "remote_worker")
        .or_else(|| {
            parsed.as_ref().and_then(|value| {
                value
                    .get("remote_worker")
                    .and_then(serde_json::Value::as_bool)
            })
        })
        .unwrap_or(false);

    Ok(RemoteSubmitAck { remote_worker })
}

fn build_remote_submit_payload(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
    include_workspace_archive: bool,
) -> Result<serde_json::Value> {
    if !include_workspace_archive {
        return Ok(serde_json::json!({
            "task_run_id": task_run_id,
            "attempt": attempt,
            "task_label": task_label,
            "selected_node_id": target.node_id,
            "workspace": {
                "mode": "REPO_ZIP_SNAPSHOT",
                "manifest_hash": remote_workspace.manifest_hash,
            },
        }));
    }

    let runtime = target
        .runtime
        .as_ref()
        .map(remote_runtime_submit_value)
        .unwrap_or(serde_json::Value::Null);

    let steps = serde_json::to_value(&task.steps)
        .context("failed serializing task steps for remote submit payload")?;

    Ok(serde_json::json!({
        "task_run_id": task_run_id,
        "attempt": attempt,
        "task_label": task_label,
        "selected_node_id": target.node_id,
        "workspace": {
            "mode": "REPO_ZIP_SNAPSHOT",
            "archive_zip_base64": remote_workspace.archive_zip_base64,
            "manifest_hash": remote_workspace.manifest_hash,
        },
        "execution": {
            "steps": steps,
            "timeout_s": task.timeout_s,
            "runtime": runtime,
        },
        "result": {
            "sync_mode": "OUTPUTS_AND_LOGS",
        },
    }))
}

fn remote_runtime_submit_value(runtime: &RemoteRuntimeSpec) -> serde_json::Value {
    match runtime {
        RemoteRuntimeSpec::Containerized { image } => serde_json::json!({
            "kind": "containerized",
            "image": image,
        }),
    }
}

/// Opens the remote event stream endpoint for one attempt.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_events(
    target: &StrictRemoteTarget,
    task_run_id: &str,
) -> Result<Vec<RemoteLogChunk>> {
    const MAX_EVENT_RECONNECTS: u32 = 3;
    const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(50);
    let max_event_wait = remote_events_max_wait_duration();

    let mut last_seen_seq = 0_u64;
    let mut reconnect_attempts = 0_u32;
    let mut persisted_remote_logs = Vec::new();
    let deadline = tokio::time::Instant::now() + max_event_wait;

    while tokio::time::Instant::now() < deadline {
        let path = format!("/v1/tasks/{task_run_id}/events?after_seq={last_seen_seq}");
        let response = remote_protocol_http_request(
            target,
            "GET",
            &path,
            None,
            "events",
            Duration::from_secs(1),
        )
        .await;

        let (status, response_body) = match response {
            Ok(success) => {
                reconnect_attempts = 0;
                success
            }
            Err(err) => {
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_EVENT_RECONNECTS {
                    bail!(
                        "infra error: remote node {} events stream resume failed after seq {}: {err}",
                        target.node_id,
                        last_seen_seq
                    );
                }
                tokio::time::sleep(EVENT_POLL_INTERVAL).await;
                continue;
            }
        };

        if status != 200 {
            bail!(
                "infra error: remote node {} events stream failed with HTTP {}",
                target.node_id,
                status
            );
        }

        let parsed = parse_remote_events_response(target, &response_body, last_seen_seq)?;
        last_seen_seq = parsed.next_seq;
        persisted_remote_logs.extend(parsed.remote_logs);
        if parsed.done {
            return Ok(persisted_remote_logs);
        }
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
    }

    bail!(
        "infra error: remote node {} events stream exceeded {}s without terminal completion",
        target.node_id,
        max_event_wait.as_secs()
    );
}

/// Parses one remote events response envelope and advances checkpoint sequence monotonically.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn parse_remote_events_response(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_body)
        && is_wrapped_remote_events_payload(&parsed)
    {
        return parse_wrapped_remote_events(target, &parsed, last_seen_seq);
    }

    parse_ndjson_remote_events(target, response_body, last_seen_seq)
}

fn is_wrapped_remote_events_payload(parsed: &serde_json::Value) -> bool {
    parsed
        .as_object()
        .is_some_and(|object| object.contains_key("events") || object.contains_key("done"))
}

fn parse_wrapped_remote_events(
    target: &StrictRemoteTarget,
    parsed: &serde_json::Value,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    if let Some(events) = parsed.get("events") {
        let events = events.as_array().ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} events payload must contain an array",
                target.node_id
            )
        })?;
        for event in events {
            let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            if seq > checkpoint {
                checkpoint = seq;
            }
            if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
                continue;
            }

            let is_log_chunk = event
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| kind == "TASK_LOG_CHUNK");
            if !is_log_chunk {
                continue;
            }

            let chunk = event
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| event.get("message").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed
            .get("done")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        remote_logs,
    })
}

fn parse_ndjson_remote_events(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    let mut done = false;

    for line in response_body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let event: serde_json::Value = serde_json::from_str(line).with_context(|| {
            format!(
                "infra error: remote node {} returned invalid NDJSON event line",
                target.node_id
            )
        })?;
        let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        if seq > checkpoint {
            checkpoint = seq;
        }
        if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
            continue;
        }

        let event_type = event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                event
                    .get("payload")
                    .and_then(|payload| payload.get("kind"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or_default();
        if event_type == "TASK_LOG_CHUNK" {
            let payload = event.get("payload").unwrap_or(&serde_json::Value::Null);
            let chunk = payload
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| payload.get("message").and_then(serde_json::Value::as_str))
                .or_else(|| event.get("chunk").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
        if matches!(
            event_type,
            "TASK_COMPLETED" | "TASK_FAILED" | "TASK_TERMINAL"
        ) {
            done = true;
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done,
        remote_logs,
    })
}

/// Fetches terminal result metadata for one remote attempt.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
) -> Result<RemoteProtocolResult> {
    let _ = attempt;
    let path = format!("/v1/tasks/{task_run_id}/result");
    let (status, response_body) =
        remote_protocol_http_request(target, "GET", &path, None, "result", Duration::from_secs(1))
            .await?;

    if status != 200 {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP {}",
            target.node_id,
            status
        );
    }

    let parsed: serde_json::Value = serde_json::from_str(&response_body).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid JSON for result",
            target.node_id
        )
    })?;
    let success = parsed
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} result missing boolean success field",
                target.node_id
            )
        })?;
    let exit_code = parsed
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok());

    if let Some(sync_mode) = parsed.get("sync_mode").and_then(serde_json::Value::as_str)
        && sync_mode != "OUTPUTS_AND_LOGS"
    {
        bail!(
            "infra error: remote node {} result sync mode `{sync_mode}` is unsupported in V1; expected `OUTPUTS_AND_LOGS`",
            target.node_id
        );
    }

    let synced_outputs = parse_remote_result_outputs(target, &parsed)?;
    Ok(RemoteProtocolResult {
        success,
        exit_code,
        synced_outputs,
        runtime_kind: parsed
            .get("runtime")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        runtime_engine: parsed
            .get("runtime_engine")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    })
}

fn parse_remote_result_outputs(
    target: &StrictRemoteTarget,
    result: &serde_json::Value,
) -> Result<Vec<SyncedOutput>> {
    let Some(outputs) = result.get("outputs") else {
        return Ok(Vec::new());
    };
    let outputs = outputs.as_array().ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} result outputs field must be an array",
            target.node_id
        )
    })?;

    let mut synced_outputs = Vec::with_capacity(outputs.len());
    for output in outputs {
        let path = output
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string path",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if path.is_empty() {
            bail!(
                "infra error: remote node {} result output path cannot be empty",
                target.node_id
            );
        }

        let digest = output
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing string digest",
                    target.node_id
                )
            })?
            .trim()
            .to_string();
        if digest.is_empty() {
            bail!(
                "infra error: remote node {} result output digest cannot be empty",
                target.node_id
            );
        }

        let size_bytes = output
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                anyhow!(
                    "infra error: remote node {} result output is missing numeric size",
                    target.node_id
                )
            })?;

        synced_outputs.push(SyncedOutput {
            path,
            digest,
            size_bytes,
        });
    }

    Ok(synced_outputs)
}

/// Sends a small HTTP request to a remote endpoint and returns `(status_code, body)`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_http_request(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: Option<&str>,
    phase: &str,
    timeout: Duration,
) -> Result<(u16, String)> {
    let socket_addr = TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;
    let header_block = remote_protocol_request_headers(target)?;
    let payload = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {socket_addr}\r\nConnection: close\r\n{header_block}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    );

    let exchange = async {
        let mut stream = TransportFactory::connect(target).await?;
        stream.write_all(request.as_bytes()).await?;
        stream.flush().await?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<Vec<u8>, anyhow::Error>(response)
    };

    let effective_timeout = TransportFactory::phase_timeout(target, timeout);
    let response_bytes = tokio::time::timeout(effective_timeout, exchange)
        .await
        .map_err(|_| {
            anyhow!(
                "infra error: remote node {} {} request timed out",
                target.node_id,
                phase
            )
        })??;

    let response_text = String::from_utf8_lossy(&response_bytes);
    let (head, body) = response_text.split_once("\r\n\r\n").ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} returned malformed HTTP response for {}",
            target.node_id,
            phase
        )
    })?;
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} returned invalid HTTP status for {}",
                target.node_id,
                phase
            )
        })?;

    Ok((status_code, body.to_string()))
}

fn remote_protocol_request_headers(target: &StrictRemoteTarget) -> Result<String> {
    let mut headers = String::from("X-Tak-Protocol-Version: v1\r\n");

    if let Some(env_name) = target.service_auth_env.as_deref() {
        let token = env::var(env_name).with_context(|| {
            format!(
                "infra error: remote node {} missing service auth token env {}",
                target.node_id, env_name
            )
        })?;
        let token = token.trim();
        if token.is_empty() {
            bail!(
                "infra error: remote node {} service auth token env {} is empty",
                target.node_id,
                env_name
            );
        }
        if token.contains(['\r', '\n']) {
            bail!(
                "infra error: remote node {} service auth token env {} contains invalid characters",
                target.node_id,
                env_name
            );
        }
        headers.push_str(&format!("X-Tak-Service-Token: {token}\r\n"));
    }

    Ok(headers)
}

/// Converts an HTTP(S) endpoint string into a connectable `host:port` address.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim();
    let (scheme, without_scheme) = if let Some(value) = trimmed.strip_prefix("http://") {
        ("http", value)
    } else if let Some(value) = trimmed.strip_prefix("https://") {
        ("https", value)
    } else {
        ("", trimmed)
    };

    let authority_end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    let authority_with_userinfo = without_scheme[..authority_end].trim();
    let authority = authority_with_userinfo
        .rsplit_once('@')
        .map_or(authority_with_userinfo, |(_, value)| value)
        .trim();
    if authority.is_empty() {
        bail!("missing host:port");
    }

    if authority.contains(':') {
        return Ok(authority.to_string());
    }

    if scheme.is_empty() {
        bail!("missing port in endpoint authority");
    }

    let default_port = if scheme == "https" { "443" } else { "80" };
    Ok(format!("{authority}:{default_port}"))
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    let socket_addr = endpoint_socket_addr(endpoint)?;
    let (host, raw_port) = socket_addr
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("missing host:port"))?;
    if host.trim().is_empty() {
        bail!("missing host");
    }
    let port = raw_port
        .parse::<u16>()
        .with_context(|| format!("invalid port `{raw_port}`"))?;
    Ok((host.to_string(), port))
}

fn test_tor_onion_dial_addr() -> Option<String> {
    env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Executes all steps in one task attempt and short-circuits on first failing step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn run_task_steps(
    task: &ResolvedTask,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<StepRunResult> {
    for step in &task.steps {
        let status = run_step(step, task.timeout_s, workspace_root, runtime_env).await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
    })
}

async fn run_task_steps_with_runtime(
    task: &ResolvedTask,
    workspace_root: &Path,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
) -> Result<StepRunResult> {
    if let Some(metadata) = runtime_metadata
        && let Some(plan) = metadata.container_plan.as_ref()
    {
        return run_task_steps_in_container(
            task,
            workspace_root,
            plan,
            Some(&metadata.env_overrides),
        )
        .await;
    }

    run_task_steps(
        task,
        workspace_root,
        runtime_metadata.map(|metadata| &metadata.env_overrides),
    )
    .await
}

pub async fn execute_remote_worker_steps(
    workspace_root: &Path,
    spec: &RemoteWorkerExecutionSpec,
) -> Result<RemoteWorkerExecutionResult> {
    let task = ResolvedTask {
        label: TaskLabel {
            package: "//".to_string(),
            name: "remote_worker_task".to_string(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: spec.steps.clone(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: spec.timeout_s,
        context: tak_core::model::CurrentStateSpec::default(),
        execution: TaskExecutionSpec::LocalOnly(tak_core::model::LocalSpec::default()),
        tags: Vec::new(),
    };

    let runtime_metadata = match spec.runtime.as_ref() {
        Some(runtime) => resolve_runtime_execution_metadata_for_node_runtime(
            &task,
            &spec.node_id,
            Some(runtime),
        )?,
        None => None,
    };

    let result =
        run_task_steps_with_runtime(&task, workspace_root, runtime_metadata.as_ref()).await?;
    Ok(RemoteWorkerExecutionResult {
        success: result.success,
        exit_code: result.exit_code,
        runtime_kind: runtime_metadata
            .as_ref()
            .map(|metadata| metadata.kind.clone()),
        runtime_engine: runtime_metadata
            .as_ref()
            .and_then(|metadata| metadata.engine.clone()),
    })
}

#[derive(Debug)]
struct ContainerStepSpec {
    argv: Vec<String>,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
}

async fn run_task_steps_in_container(
    task: &ResolvedTask,
    workspace_root: &Path,
    plan: &ContainerExecutionPlan,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<StepRunResult> {
    let docker = connect_container_engine(plan.engine).await?;
    ensure_container_image(&docker, &plan.image).await?;

    for step in &task.steps {
        let step_spec = build_container_step_spec(step, workspace_root, runtime_env)?;
        let status = run_step_in_container(
            &docker,
            &plan.image,
            &step_spec,
            task.timeout_s,
            workspace_root,
        )
        .await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
    })
}

async fn connect_container_engine(engine: ContainerEngine) -> Result<Docker> {
    let docker = match engine {
        ContainerEngine::Docker => Docker::connect_with_local_defaults().context(
            "infra error: container lifecycle start failed: docker client connect failed",
        )?,
        ContainerEngine::Podman => connect_podman_client()?,
    };
    docker.ping().await.with_context(|| {
        format!(
            "infra error: container lifecycle start failed: {} ping failed",
            engine_name(engine)
        )
    })?;
    Ok(docker)
}

fn connect_podman_client() -> Result<Docker> {
    for socket in podman_socket_candidates_from_env() {
        let socket_path = socket.strip_prefix("unix://").unwrap_or(socket.as_str());
        if let Ok(client) =
            Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)
        {
            return Ok(client);
        }
    }
    bail!("infra error: container lifecycle start failed: no podman socket available");
}

async fn ensure_container_image(docker: &Docker, image: &str) -> Result<()> {
    match docker.inspect_image(image).await {
        Ok(_) => return Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(err) => {
            return Err(err)
                .context("infra error: container lifecycle pull failed: inspect image failed");
        }
    }

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(item) = stream.next().await {
        item.context("infra error: container lifecycle pull failed")?;
    }
    Ok(())
}

fn build_container_step_spec(
    step: &StepDef,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<ContainerStepSpec> {
    match step {
        StepDef::Cmd { argv, cwd, env } => {
            if argv.is_empty() {
                bail!("cmd step requires a non-empty argv");
            }
            let mut env_map = BTreeMap::new();
            if let Some(runtime_env) = runtime_env {
                env_map.extend(runtime_env.clone());
            }
            env_map.extend(env.clone());
            Ok(ContainerStepSpec {
                argv: argv.clone(),
                cwd: resolve_cwd(workspace_root, cwd),
                env: env_map,
            })
        }
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => {
            let mut full_argv = Vec::with_capacity(argv.len() + 2);
            if let Some(interpreter) = interpreter {
                full_argv.push(interpreter.clone());
                full_argv.push(path.clone());
            } else {
                full_argv.push(path.clone());
            }
            full_argv.extend(argv.clone());

            let mut env_map = BTreeMap::new();
            if let Some(runtime_env) = runtime_env {
                env_map.extend(runtime_env.clone());
            }
            env_map.extend(env.clone());
            Ok(ContainerStepSpec {
                argv: full_argv,
                cwd: resolve_cwd(workspace_root, cwd),
                env: env_map,
            })
        }
    }
}

async fn run_step_in_container(
    docker: &Docker,
    image: &str,
    step: &ContainerStepSpec,
    timeout_s: Option<u64>,
    workspace_root: &Path,
) -> Result<StepRunResult> {
    let container_name = format!("tak-step-{}", Uuid::new_v4());
    let bind_mount = format!(
        "{}:{}:rw",
        workspace_root.display(),
        workspace_root.display()
    );
    let env = step
        .env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();

    let config = ContainerConfig {
        image: Some(image.to_string()),
        cmd: Some(step.argv.clone()),
        env: Some(env),
        working_dir: Some(step.cwd.to_string_lossy().to_string()),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        host_config: Some(HostConfig {
            binds: Some(vec![bind_mount]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            config,
        )
        .await
        .context("infra error: container lifecycle start failed: create container failed")?;
    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await
        .context("infra error: container lifecycle start failed: start container failed")?;

    let wait_result = wait_for_container_exit_code(docker, &container_name);
    let status = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), wait_result).await {
            Ok(result) => result?,
            Err(_) => {
                let _ = cleanup_container(docker, &container_name).await;
                return Ok(StepRunResult {
                    success: false,
                    exit_code: None,
                });
            }
        }
    } else {
        wait_result.await?
    };

    let _ = cleanup_container(docker, &container_name).await;
    Ok(StepRunResult {
        success: status == 0,
        exit_code: Some(status),
    })
}

async fn wait_for_container_exit_code(docker: &Docker, container_name: &str) -> Result<i32> {
    let mut stream = docker.wait_container(container_name, None::<WaitContainerOptions<String>>);
    let Some(result) = stream.next().await else {
        bail!("infra error: container lifecycle runtime failed: wait stream ended unexpectedly");
    };
    let result = result
        .context("infra error: container lifecycle runtime failed: waiting for container failed")?;
    let code = i32::try_from(result.status_code).unwrap_or(1);
    Ok(code)
}

async fn cleanup_container(docker: &Docker, container_name: &str) -> Result<()> {
    let _ = docker
        .remove_container(
            container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    Ok(())
}

#[cfg(test)]
mod lib_tests;
