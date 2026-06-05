use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::retry::retry_backoff_delay;

use super::super::super::super::output_observer::{
    TaskStatusDetails, emit_task_status_message_with_details,
};
use super::super::super::super::remote_submit_failure::RemoteSubmitFailure;
use super::super::super::super::{RunOptions, TaskStatusEventKind, TaskStatusPhase};
use super::StartedAttemptContext;

const REMOTE_SETUP_INFRA_ATTEMPTS: u32 = 3;

pub(super) fn can_retry_remote_setup(
    task: &ResolvedTask,
    attempt: u32,
    error: &anyhow::Error,
) -> bool {
    attempt < remote_setup_attempts(task) && is_retryable_remote_setup_failure(error)
}

pub(super) async fn wait_before_remote_setup_retry(
    task: &ResolvedTask,
    options: &RunOptions,
    attempt: u32,
    context: &StartedAttemptContext<'_>,
    error: &anyhow::Error,
) -> Result<()> {
    let wait = retry_backoff_delay(&task.retry.backoff, attempt);
    emit_retry_status(task, options, attempt, context, error, wait)?;
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
    Ok(())
}

fn remote_setup_attempts(task: &ResolvedTask) -> u32 {
    REMOTE_SETUP_INFRA_ATTEMPTS.max(task.retry.attempts.max(1))
}

fn is_retryable_remote_setup_failure(error: &anyhow::Error) -> bool {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<RemoteSubmitFailure>())
        .is_some_and(RemoteSubmitFailure::is_retryable)
}

fn emit_retry_status(
    task: &ResolvedTask,
    options: &RunOptions,
    attempt: u32,
    context: &StartedAttemptContext<'_>,
    error: &anyhow::Error,
    wait: std::time::Duration,
) -> Result<()> {
    emit_task_status_message_with_details(
        options.output_observer.as_ref(),
        &task.label,
        attempt + 1,
        TaskStatusPhase::RetryWait,
        context.placement.remote_node_id.as_deref(),
        format!(
            "retrying remote setup after retryable infra failure {}",
            wait_text(wait)
        ),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::RetryScheduled),
            original_error: Some(error.to_string()),
            retryable: Some(true),
            transport: context
                .placement
                .strict_remote_target
                .as_ref()
                .map(|target| target.transport_kind.as_result_value().to_string()),
            ..TaskStatusDetails::default()
        },
    )
}

fn wait_text(wait: std::time::Duration) -> String {
    if wait.is_zero() {
        return "immediately".to_string();
    }
    format!("in {wait:?}")
}
