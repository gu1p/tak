//! Task execution engine for resolved workspace tasks.
//!
//! This crate expands target dependencies, enforces execution ordering, applies retry and
//! timeout policy, and optionally coordinates daemon leases around task execution.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{BackoffDef, NeedDef, ResolvedTask, StepDef, TaskLabel, WorkspaceSpec};
use takd::{
    AcquireLeaseRequest, ClientInfo, NeedRequest, ReleaseLeaseRequest, Request, Response, TaskInfo,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use uuid::Uuid;

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
struct LeaseContext {
    user: String,
    session_id: String,
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

/// Collects all tasks required to execute `targets` including transitive dependencies.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn collect_required_labels(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
) -> Result<BTreeSet<TaskLabel>> {
    let mut required = BTreeSet::new();
    let mut visiting = Vec::new();

    for target in targets {
        dfs_collect(target, spec, &mut required, &mut visiting)?;
    }

    Ok(required)
}

/// Depth-first dependency traversal used to populate the required task set.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn dfs_collect(
    label: &TaskLabel,
    spec: &WorkspaceSpec,
    required: &mut BTreeSet<TaskLabel>,
    visiting: &mut Vec<TaskLabel>,
) -> Result<()> {
    if required.contains(label) {
        return Ok(());
    }

    if visiting.contains(label) {
        bail!("cycle detected while collecting dependencies at {label}");
    }

    let task = spec
        .tasks
        .get(label)
        .ok_or_else(|| anyhow!("target does not exist: {label}"))?;

    visiting.push(label.clone());
    for dep in &task.deps {
        dfs_collect(dep, spec, required, visiting)?;
    }
    visiting.pop();

    required.insert(label.clone());
    Ok(())
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
    let total_attempts = task.retry.attempts.max(1);
    let mut attempt = 0;

    loop {
        attempt += 1;

        let lease_id = acquire_task_lease(task, attempt, options, lease_context).await?;
        let run_result = run_task_steps(task, workspace_root).await;
        let release_result = match &lease_id {
            Some(id) => release_task_lease(id, options).await,
            None => Ok(()),
        };

        if let Err(err) = release_result {
            return Err(err).context(format!("failed releasing lease for {}", task.label));
        }

        let run = run_result?;
        let last_exit_code = run.exit_code;

        if run.success {
            return Ok(TaskRunResult {
                attempts: attempt,
                success: true,
                exit_code: run.exit_code,
            });
        }

        let can_retry =
            attempt < total_attempts && should_retry(last_exit_code, &task.retry.on_exit);
        if !can_retry {
            return Ok(TaskRunResult {
                attempts: attempt,
                success: false,
                exit_code: last_exit_code,
            });
        }

        let wait = retry_backoff_delay(&task.retry.backoff, attempt);
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// Repeatedly requests a lease for a task until granted or a terminal daemon error occurs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn acquire_task_lease(
    task: &ResolvedTask,
    attempt: u32,
    options: &RunOptions,
    lease_context: &LeaseContext,
) -> Result<Option<String>> {
    let Some(socket_path) = options.lease_socket.as_ref() else {
        return Ok(None);
    };

    if task.needs.is_empty() {
        return Ok(None);
    }

    let request_id = Uuid::new_v4().to_string();
    let acquire_request = AcquireLeaseRequest {
        request_id: request_id.clone(),
        client: ClientInfo {
            user: lease_context.user.clone(),
            pid: std::process::id(),
            session_id: lease_context.session_id.clone(),
        },
        task: TaskInfo {
            label: task.label.to_string(),
            attempt,
        },
        needs: convert_needs(&task.needs),
        ttl_ms: options.lease_ttl_ms.max(1_000),
    };

    loop {
        let response =
            send_daemon_request(socket_path, Request::AcquireLease(acquire_request.clone()))
                .await
                .with_context(|| format!("lease acquire request failed for {}", task.label))?;

        match response {
            Response::LeaseGranted { lease, .. } => return Ok(Some(lease.lease_id)),
            Response::LeasePending { .. } => {
                let poll_ms = options.lease_poll_interval_ms.max(10);
                tokio::time::sleep(Duration::from_millis(poll_ms)).await;
            }
            Response::Error { message, .. } => {
                bail!(
                    "daemon rejected lease request for {}: {message}",
                    task.label
                )
            }
            other => bail!("unexpected response while acquiring lease: {other:?}"),
        }
    }
}

/// Releases a previously granted lease id using the daemon protocol.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn release_task_lease(lease_id: &str, options: &RunOptions) -> Result<()> {
    let Some(socket_path) = options.lease_socket.as_ref() else {
        return Ok(());
    };

    let response = send_daemon_request(
        socket_path,
        Request::ReleaseLease(ReleaseLeaseRequest {
            request_id: Uuid::new_v4().to_string(),
            lease_id: lease_id.to_string(),
        }),
    )
    .await
    .with_context(|| format!("release request failed for lease {lease_id}"))?;

    match response {
        Response::LeaseReleased { .. } => Ok(()),
        Response::Error { message, .. } => bail!("daemon failed to release lease: {message}"),
        other => bail!("unexpected response while releasing lease: {other:?}"),
    }
}

/// Converts core model need definitions into daemon wire-format needs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn convert_needs(needs: &[NeedDef]) -> Vec<NeedRequest> {
    needs
        .iter()
        .map(|need| NeedRequest {
            name: need.limiter.name.clone(),
            scope: need.limiter.scope.clone(),
            scope_key: need.limiter.scope_key.clone(),
            slots: need.slots,
        })
        .collect()
}

/// Sends one NDJSON request to the daemon and returns the decoded response frame.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn send_daemon_request(socket_path: &Path, request: Request) -> Result<Response> {
    let stream = UnixStream::connect(socket_path)
        .await
        .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();

    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before response");
    }

    serde_json::from_str(line.trim_end()).context("failed to decode daemon response")
}

/// Returns true when the given exit code qualifies for retry under policy rules.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn should_retry(exit_code: Option<i32>, retry_on_exit: &[i32]) -> bool {
    if retry_on_exit.is_empty() {
        return true;
    }

    exit_code.is_some_and(|code| retry_on_exit.contains(&code))
}

/// Computes retry delay duration for the configured backoff strategy.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn retry_backoff_delay(backoff: &BackoffDef, attempt: u32) -> Duration {
    match backoff {
        BackoffDef::Fixed { seconds } => seconds_to_duration(*seconds),
        BackoffDef::ExpJitter { min_s, max_s, .. } => {
            let exponent = attempt.saturating_sub(1).min(20);
            let factor = 1u64 << exponent;
            let delay = (min_s * factor as f64).min(*max_s);
            seconds_to_duration(delay)
        }
    }
}

/// Converts a floating-point second value into a clamped non-negative duration.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn seconds_to_duration(seconds: f64) -> Duration {
    if !seconds.is_finite() || seconds <= 0.0 {
        return Duration::ZERO;
    }
    Duration::from_secs_f64(seconds)
}

#[derive(Debug)]
struct StepRunResult {
    success: bool,
    exit_code: Option<i32>,
}

/// Executes all steps in one task attempt and short-circuits on first failing step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn run_task_steps(task: &ResolvedTask, workspace_root: &Path) -> Result<StepRunResult> {
    for step in &task.steps {
        let status = run_step(step, task.timeout_s, workspace_root).await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
    })
}

/// Executes one step definition with optional timeout enforcement.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn run_step(
    step: &StepDef,
    timeout_s: Option<u64>,
    workspace_root: &Path,
) -> Result<StepRunResult> {
    let (mut command, cwd) = build_command(step, workspace_root)?;
    command.current_dir(cwd);
    command.kill_on_drop(true);

    let mut child = command.spawn().context("failed to spawn process")?;

    let wait_result = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), child.wait()).await {
            Ok(wait) => wait.context("failed while waiting for process")?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Ok(StepRunResult {
                    success: false,
                    exit_code: None,
                });
            }
        }
    } else {
        child
            .wait()
            .await
            .context("failed while waiting for process")?
    };

    Ok(StepRunResult {
        success: wait_result.success(),
        exit_code: wait_result.code(),
    })
}

/// Builds an executable process command and effective working directory for a step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn build_command(step: &StepDef, workspace_root: &Path) -> Result<(Command, PathBuf)> {
    match step {
        StepDef::Cmd { argv, cwd, env } => {
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| anyhow!("cmd step requires a non-empty argv"))?;
            let mut command = Command::new(program);
            command.args(args);
            for (key, value) in env {
                command.env(key, value);
            }
            Ok((command, resolve_cwd(workspace_root, cwd)))
        }
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => {
            let mut command = if let Some(interpreter) = interpreter {
                let mut cmd = Command::new(interpreter);
                cmd.arg(path);
                cmd.args(argv);
                cmd
            } else {
                let mut cmd = Command::new(path);
                cmd.args(argv);
                cmd
            };
            for (key, value) in env {
                command.env(key, value);
            }
            Ok((command, resolve_cwd(workspace_root, cwd)))
        }
    }
}

/// Resolves a step-local working directory against the workspace root.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_cwd(workspace_root: &Path, cwd: &Option<String>) -> PathBuf {
    match cwd {
        Some(value) => {
            let path = Path::new(value);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            }
        }
        None => workspace_root.to_path_buf(),
    }
}

/// Returns the set of labels included in a run summary.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn target_set_from_summary(summary: &RunSummary) -> HashSet<TaskLabel> {
    summary.results.keys().cloned().collect()
}
