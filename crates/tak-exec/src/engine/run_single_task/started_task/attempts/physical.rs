use std::path::Path;

use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use crate::engine::attempt_submit::{AttemptSubmitState, resolve_attempt_submit_state};
use crate::engine::remote_failover::prepare_remote_failover;
use crate::engine::remote_failure::requires_worker_failover;
use crate::engine::remote_selection::SharedRemoteSelectionState;
use crate::engine::{RunOptions, is_run_cancelled_error};

use super::StartedAttemptContext;

pub(super) async fn run_physical_attempts(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    authored_attempt: u32,
    context: &mut StartedAttemptContext<'_>,
    selection_state: &SharedRemoteSelectionState,
) -> Result<AttemptExecutionOutcome> {
    loop {
        let result = submit_and_run(
            task,
            workspace_root,
            options,
            authored_attempt,
            context,
            selection_state,
        )
        .await;
        match result {
            Ok(outcome) if requires_worker_failover(outcome.remote_failure_kind) => {
                let cause = outcome.failure_detail.clone().unwrap_or_else(|| {
                    format!(
                        "remote task failed with exit code {:?}",
                        outcome.last_exit_code
                    )
                });
                fail_over(
                    task,
                    options,
                    authored_attempt,
                    context,
                    selection_state,
                    cause,
                    outcome.remote_failure_kind,
                )?;
            }
            Ok(outcome) => return Ok(outcome),
            Err(error) if is_run_cancelled_error(&error) => return Err(error),
            Err(error) => fail_over(
                task,
                options,
                authored_attempt,
                context,
                selection_state,
                error.to_string(),
                Some(crate::engine::remote_failure::RemoteFailureKind::Infrastructure),
            )?,
        }
    }
}

async fn submit_and_run(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    attempt: u32,
    context: &mut StartedAttemptContext<'_>,
    selection_state: &SharedRemoteSelectionState,
) -> Result<AttemptExecutionOutcome> {
    resolve_attempt_submit_state(
        task,
        workspace_root,
        &mut *context.placement,
        AttemptSubmitState {
            remote_workspace: context.remote_workspace,
            workspace_content_hash: context.workspace_content_hash,
            task_run_id: context.task_run_id,
            attempt,
            session: context.session,
            fused_members: None,
            execution_label: context.execution_label,
            fused_member_execution_labels: None,
        },
        options.output_observer.as_ref(),
        &options.cancellation,
        selection_state,
    )
    .await?;
    execute_task_attempt(&AttemptExecutionContext {
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
    })
    .await
}

fn fail_over(
    task: &ResolvedTask,
    options: &RunOptions,
    attempt: u32,
    context: &mut StartedAttemptContext<'_>,
    selection_state: &SharedRemoteSelectionState,
    cause: String,
    failure_kind: Option<crate::engine::remote_failure::RemoteFailureKind>,
) -> Result<()> {
    prepare_remote_failover(
        context.placement,
        cause,
        failure_kind,
        selection_state,
        options.output_observer.as_ref(),
        &task.label,
        attempt,
    )
}
