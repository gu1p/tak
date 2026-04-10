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

use crate::{OutputStream, TaskOutputObserver};

#[derive(Debug)]
pub(crate) struct StepRunResult {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
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
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
    task_label: &TaskLabel,
    attempt: u32,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
) -> Result<StepRunResult> {
    let (mut command, cwd) = build_command(step, workspace_root, runtime_env)?;
    command.current_dir(cwd);
    command.kill_on_drop(true);
    if output_observer.is_some() {
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
    }

    let mut child = command.spawn().context("failed to spawn process")?;
    let relay_observer = output_observer.cloned();
    let stdout_task = spawn_output_relay(
        child.stdout.take(),
        task_label.clone(),
        attempt,
        OutputStream::Stdout,
        relay_observer.clone(),
    );
    let stderr_task = spawn_output_relay(
        child.stderr.take(),
        task_label.clone(),
        attempt,
        OutputStream::Stderr,
        relay_observer,
    );

    let wait_result = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), child.wait()).await {
            Ok(wait) => wait.context("failed while waiting for process")?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = finish_output_relays(stdout_task, stderr_task).await;
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
    finish_output_relays(stdout_task, stderr_task).await?;

    Ok(StepRunResult {
        success: wait_result.success(),
        exit_code: wait_result.code(),
    })
}

type OutputRelayTask = Option<tokio::task::JoinHandle<Result<()>>>;
include!("step_runner/output_relay.rs");

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
