use std::path::Path;

use anyhow::{Context, Result};
use tak_core::model::ResolvedTask;

use crate::lease_client::{TaskLease, acquire_task_lease, release_task_lease};
use crate::retry::{retry_backoff_delay, should_retry};

use super::super::super::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use super::super::super::attempt_submit::{AttemptSubmitState, resolve_attempt_submit_state};
use super::super::super::output_observer::emit_task_status_message;
use super::super::super::remote_models::{
    RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement,
};
use super::super::super::remote_selection::SharedRemoteSelectionState;
use super::super::super::session_workspaces::{PreparedTaskSession, SharedExecutionSessionManager};
use super::super::super::task_result::{TaskRunResultContext, build_task_run_result};
use super::super::super::{
    LeaseContext, PlacementMode, RunOptions, TaskRunResult, TaskStatusPhase,
};
use super::super::events::emit_finished;

pub(super) struct StartedAttemptContext<'a> {
    pub(super) task_run_id: &'a str,
    pub(super) placement: &'a mut TaskPlacement,
    pub(super) attempt: &'a mut u32,
    pub(super) runtime_metadata: Option<&'a RuntimeExecutionMetadata>,
    pub(super) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(super) session: Option<&'a PreparedTaskSession>,
    pub(super) run_root: &'a Path,
    pub(super) execution_label: Option<&'a str>,
}

pub(super) async fn run_attempts(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &SharedExecutionSessionManager,
    remote_selection_state: &SharedRemoteSelectionState,
    context: StartedAttemptContext<'_>,
) -> Result<TaskRunResult> {
    let mut context = context;
    loop {
        *context.attempt += 1;
        let current_attempt = *context.attempt;
        let mut lease = acquire_task_lease(task, current_attempt, options, lease_context).await?;
        let attempt_result = async {
            submit_remote_attempt_if_needed(
                task,
                options,
                current_attempt,
                &mut context,
                remote_selection_state,
            )
            .await?;
            run_one_attempt(task, workspace_root, options, current_attempt, &context).await
        }
        .await;
        release_attempt_lease(lease.as_mut(), task, options).await?;
        let outcome = attempt_result?;
        if outcome.attempt_success || !can_retry(task, current_attempt, outcome.last_exit_code) {
            let result = build_task_result(current_attempt, outcome, &context);
            sessions.finish_task(context.session, result.success)?;
            emit_finished(options, task, &result)?;
            return Ok(result);
        }
        wait_before_retry(task, options, current_attempt, &context).await?;
    }
}

async fn submit_remote_attempt_if_needed(
    task: &ResolvedTask,
    options: &RunOptions,
    attempt: u32,
    context: &mut StartedAttemptContext<'_>,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<()> {
    resolve_attempt_submit_state(
        task,
        &mut *context.placement,
        AttemptSubmitState {
            remote_workspace: context.remote_workspace,
            task_run_id: context.task_run_id,
            attempt,
            session: context.session,
            fused_members: None,
            execution_label: context.execution_label,
            fused_member_execution_labels: None,
        },
        options.output_observer.as_ref(),
        &options.cancellation,
        remote_selection_state,
    )
    .await
}

async fn run_one_attempt(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    attempt: u32,
    context: &StartedAttemptContext<'_>,
) -> Result<AttemptExecutionOutcome> {
    let attempt_context = AttemptExecutionContext {
        task,
        workspace_root,
        run_root: context.run_root,
        placement: &*context.placement,
        runtime_metadata: context.runtime_metadata,
        remote_workspace: context.remote_workspace,
        task_run_id: context.task_run_id,
        attempt,
        output_observer: options.output_observer.as_ref(),
        cancellation: &options.cancellation,
    };
    execute_task_attempt(&attempt_context).await
}

async fn release_attempt_lease(
    lease: Option<&mut TaskLease>,
    task: &ResolvedTask,
    options: &RunOptions,
) -> Result<()> {
    if let Some(lease) = lease {
        lease.stop_renewal();
        release_task_lease(lease.id(), options)
            .await
            .context(format!("failed releasing lease for {}", task.label))?;
    }
    Ok(())
}

fn build_task_result(
    attempt: u32,
    outcome: AttemptExecutionOutcome,
    context: &StartedAttemptContext<'_>,
) -> TaskRunResult {
    build_task_run_result(
        TaskRunResultContext {
            task_run_id: context.task_run_id,
            attempt,
            success: outcome.attempt_success,
            placement: &*context.placement,
            remote_workspace: context.remote_workspace,
            runtime_metadata: context.runtime_metadata,
            session: context.session,
        },
        outcome,
    )
}

fn can_retry(task: &ResolvedTask, attempt: u32, exit_code: Option<i32>) -> bool {
    attempt < task.retry.attempts.max(1) && should_retry(exit_code, &task.retry.on_exit)
}

async fn wait_before_retry(
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
