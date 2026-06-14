use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::retry::{retry_backoff_delay, should_retry};

use super::super::super::super::output_observer::emit_task_status_message;
use super::super::super::super::{PlacementMode, RunOptions, TaskStatusPhase};
use super::StartedAttemptContext;

pub(super) fn can_retry(task: &ResolvedTask, attempt: u32, exit_code: Option<i32>) -> bool {
    attempt < task.retry.attempts.max(1) && should_retry(exit_code, &task.retry.on_exit)
}

pub(super) async fn wait_before_retry(
    task: &ResolvedTask,
    options: &RunOptions,
    attempt: u32,
    context: &StartedAttemptContext<'_>,
) -> Result<()> {
    let wait = retry_backoff_delay(&task.retry.backoff, attempt);
    if context.placement.placement_mode == PlacementMode::Remote {
        let message = if wait.is_zero() {
            "retrying after failure immediately".to_string()
        } else {
            format!("retrying after failure in {wait:?}")
        };
        emit_task_status_message(
            options.output_observer.as_ref(),
            &task.label,
            attempt + 1,
            TaskStatusPhase::RetryWait,
            context.placement.remote_node_id.as_deref(),
            message,
        )?;
    }
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
    Ok(())
}
