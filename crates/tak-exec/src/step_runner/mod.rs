use std::collections::BTreeMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use anyhow::{Context, Result};
use tak_core::model::StepDef;
use tak_core::model::TaskLabel;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::engine::cancelled_error;
use crate::engine::{ContainerExecutionIdentity, RunCancellation};
use crate::{OutputStream, TaskOutputObserver};

mod child_process;
mod command;
mod output_relay;

use child_process::{configure_child_process_group, wait_for_child};
use command::build_command;
use output_relay::{finish_output_relays, spawn_output_relay};

pub(crate) use command::resolve_cwd;

#[derive(Debug)]
pub(crate) struct StepRunResult {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) container_oom_killed: Option<bool>,
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

type OutputRelayTask = Option<tokio::task::JoinHandle<Result<()>>>;

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
                container_oom_killed: None,
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
        container_oom_killed: None,
    })
}
