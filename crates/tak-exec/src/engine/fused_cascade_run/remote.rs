use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::RunOptions;
use crate::engine::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use crate::engine::attempt_submit::{AttemptSubmitState, resolve_attempt_submit_state};
use crate::engine::fused_cascade::FusedCascade;
use crate::engine::remote_failover::prepare_remote_failover;
use crate::engine::remote_failure::requires_worker_failover;
use crate::engine::remote_models::{RemoteWorkspaceStage, TaskPlacement};
use crate::engine::remote_selection::SharedRemoteSelectionState;
use crate::engine::session_workspaces::PreparedTaskSession;

pub(super) struct RemoteFusedAttemptContext<'a> {
    pub(super) cascade: &'a FusedCascade,
    pub(super) workspace_root: &'a Path,
    pub(super) options: &'a RunOptions,
    pub(super) task_run_id: &'a str,
    pub(super) placement: &'a mut TaskPlacement,
    pub(super) remote_selection_state: &'a SharedRemoteSelectionState,
    pub(super) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(super) workspace_content_hash: Option<&'a str>,
    pub(super) session: Option<&'a PreparedTaskSession>,
    pub(super) execution_label: Option<&'a str>,
    pub(super) member_execution_labels: &'a BTreeMap<TaskLabel, String>,
}

pub(super) async fn run_remote_fused_attempt(
    mut context: RemoteFusedAttemptContext<'_>,
) -> Result<(u32, AttemptExecutionOutcome)> {
    loop {
        let result = run_physical_fused_attempt(&mut context).await;
        match result {
            Ok(outcome) if requires_worker_failover(outcome.remote_failure_kind) => {
                let cause = outcome.failure_detail.clone().unwrap_or_else(|| {
                    format!(
                        "remote fused task failed with exit code {:?}",
                        outcome.last_exit_code
                    )
                });
                prepare_remote_failover(
                    context.placement,
                    cause,
                    outcome.remote_failure_kind,
                    context.remote_selection_state,
                    context.options.output_observer.as_ref(),
                    &context.cascade.task.label,
                    1,
                )?;
            }
            Ok(outcome) => return Ok((1, outcome)),
            Err(error) if crate::engine::is_run_cancelled_error(&error) => return Err(error),
            Err(error) => prepare_remote_failover(
                context.placement,
                error.to_string(),
                Some(crate::engine::remote_failure::RemoteFailureKind::Infrastructure),
                context.remote_selection_state,
                context.options.output_observer.as_ref(),
                &context.cascade.task.label,
                1,
            )?,
        }
    }
}

async fn run_physical_fused_attempt(
    context: &mut RemoteFusedAttemptContext<'_>,
) -> Result<AttemptExecutionOutcome> {
    resolve_attempt_submit_state(
        &context.cascade.task,
        context.workspace_root,
        context.placement,
        AttemptSubmitState {
            remote_workspace: context.remote_workspace,
            workspace_content_hash: context.workspace_content_hash,
            task_run_id: context.task_run_id,
            attempt: 1,
            session: context.session,
            fused_members: Some(&context.cascade.members),
            execution_label: context.execution_label,
            fused_member_execution_labels: Some(context.member_execution_labels),
        },
        context.options.output_observer.as_ref(),
        &context.options.cancellation,
        context.remote_selection_state,
    )
    .await?;
    execute_task_attempt(&AttemptExecutionContext {
        task: &context.cascade.task,
        workspace_root: context.workspace_root,
        run_root: context.workspace_root,
        placement: context.placement,
        runtime_metadata: None,
        remote_workspace: context.remote_workspace,
        task_run_id: context.task_run_id,
        attempt: 1,
        output_observer: context.options.output_observer.as_ref(),
        cancellation: &context.options.cancellation,
    })
    .await
}
