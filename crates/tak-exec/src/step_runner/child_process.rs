//! Spawned child-process lifecycle: bounded waiting plus process-group teardown.

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::RunCancellation;
use crate::cancellation::cancelled_error;
use crate::deadline::{DeadlineOutcome, select_deadline_outcome};

pub(super) async fn wait_for_child(
    child: &mut tokio::process::Child,
    timeout_s: Option<u64>,
    cancellation: &RunCancellation,
) -> Result<Option<std::process::ExitStatus>> {
    if let Some(seconds) = timeout_s {
        let outcome = if seconds == 0 {
            select_deadline_outcome(child.wait(), std::future::ready(()), cancellation).await
        } else {
            select_deadline_outcome(
                child.wait(),
                tokio::time::sleep(Duration::from_secs(seconds)),
                cancellation,
            )
            .await
        };
        return match outcome {
            DeadlineOutcome::Cancelled => {
                kill_child(child).await;
                Err(cancelled_error())
            }
            DeadlineOutcome::TimedOut => {
                kill_child(child).await;
                Ok(None)
            }
            DeadlineOutcome::Completed(wait) => {
                Ok(Some(wait.context("failed while waiting for process")?))
            }
        };
    }
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => {
            kill_child(child).await;
            Err(cancelled_error())
        }
        wait = child.wait() => Ok(Some(wait.context("failed while waiting for process")?)),
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
