//! Spawned child-process lifecycle: bounded waiting plus process-group teardown.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::engine::{RunCancellation, cancelled_error};

pub(super) async fn wait_for_child(
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
pub(super) fn configure_child_process_group(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
pub(super) fn configure_child_process_group(_command: &mut Command) {}

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
