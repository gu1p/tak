use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use tak_core::model::StepDef;
use tak_core::model::TaskLabel;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use crate::engine::cancelled_error;
use crate::engine::{ContainerExecutionIdentity, RunCancellation};
use crate::{OutputStream, TaskOutputObserver};

mod output_relay;

use output_relay::{finish_output_relays, spawn_output_relay};

#[derive(Debug)]
pub(crate) struct StepRunResult {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
}

pub(crate) struct StepRunContext<'a> {
    pub(crate) workspace_root: &'a Path,
    pub(crate) runtime_env: Option<&'a BTreeMap<String, String>>,
    pub(crate) task_label: &'a TaskLabel,
    pub(crate) attempt: u32,
    pub(crate) task_run_id: &'a str,
    pub(crate) output_observer: Option<&'a Arc<dyn TaskOutputObserver>>,
    pub(crate) cancellation: &'a RunCancellation,
    pub(crate) container_identity: Option<&'a ContainerExecutionIdentity>,
}

/// Executes one step definition with optional timeout enforcement.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn run_step(
    step: &StepDef,
    timeout_s: Option<u64>,
    context: StepRunContext<'_>,
) -> Result<StepRunResult> {
    if context.cancellation.is_cancelled() {
        return Err(cancelled_error());
    }
    let (mut command, cwd) = build_command(step, context.workspace_root, context.runtime_env)?;
    command.current_dir(cwd);
    command.kill_on_drop(true);
    configure_child_process_group(&mut command);
    if context.output_observer.is_some() {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }

    let mut child = command.spawn().context("failed to spawn process")?;
    let relay_observer = context.output_observer.cloned();
    let stdout_task = spawn_output_relay(
        child.stdout.take(),
        context.task_label.clone(),
        context.task_run_id.to_string(),
        context.attempt,
        OutputStream::Stdout,
        relay_observer.clone(),
    );
    let stderr_task = spawn_output_relay(
        child.stderr.take(),
        context.task_label.clone(),
        context.task_run_id.to_string(),
        context.attempt,
        OutputStream::Stderr,
        relay_observer,
    );

    let wait_result = wait_for_child(&mut child, timeout_s, context.cancellation).await;
    let wait_result = match wait_result {
        Ok(Some(status)) => status,
        Ok(None) => {
            let _ = finish_output_relays(stdout_task, stderr_task).await;
            return Ok(StepRunResult {
                success: false,
                exit_code: None,
            });
        }
        Err(error) => {
            let _ = finish_output_relays(stdout_task, stderr_task).await;
            return Err(error);
        }
    };
    finish_output_relays(stdout_task, stderr_task).await?;

    Ok(StepRunResult {
        success: wait_result.success(),
        exit_code: wait_result.code(),
    })
}

async fn wait_for_child(
    child: &mut tokio::process::Child,
    timeout_s: Option<u64>,
    cancellation: &RunCancellation,
) -> Result<Option<std::process::ExitStatus>> {
    if let Some(seconds) = timeout_s {
        let timeout = tokio::time::sleep(Duration::from_secs(seconds));
        tokio::pin!(timeout);
        return tokio::select! {
            wait = child.wait() => Ok(Some(wait.context("failed while waiting for process")?)),
            _ = &mut timeout => {
                kill_child(child).await;
                Ok(None)
            }
            _ = cancellation.cancelled() => {
                kill_child(child).await;
                Err(cancelled_error())
            }
        };
    }
    tokio::select! {
        wait = child.wait() => Ok(Some(wait.context("failed while waiting for process")?)),
        _ = cancellation.cancelled() => {
            kill_child(child).await;
            Err(cancelled_error())
        }
    }
}

#[cfg(unix)]
fn configure_child_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_child_process_group(_command: &mut Command) {}

async fn kill_child(child: &mut tokio::process::Child) {
    kill_child_process_group(child).await;
    let _ = child.wait().await;
}

#[cfg(unix)]
async fn kill_child_process_group(child: &mut tokio::process::Child) {
    if let Some(pid) = child.id() {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
    let _ = child.kill().await;
}

#[cfg(not(unix))]
async fn kill_child_process_group(child: &mut tokio::process::Child) {
    let _ = child.kill().await;
}

type OutputRelayTask = Option<tokio::task::JoinHandle<Result<()>>>;

/// Builds an executable process command and effective working directory for a step.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn build_command(
    step: &StepDef,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<(Command, PathBuf)> {
    match step {
        StepDef::Cmd { argv, cwd, env } => {
            let (program, args) = argv
                .split_first()
                .ok_or_else(|| anyhow!("cmd step requires a non-empty argv"))?;
            let mut command = Command::new(program);
            command.args(args);
            if let Some(runtime_env) = runtime_env {
                for (key, value) in runtime_env {
                    command.env(key, value);
                }
            }
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
            if let Some(runtime_env) = runtime_env {
                for (key, value) in runtime_env {
                    command.env(key, value);
                }
            }
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
pub(crate) fn resolve_cwd(workspace_root: &Path, cwd: &Option<String>) -> PathBuf {
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
