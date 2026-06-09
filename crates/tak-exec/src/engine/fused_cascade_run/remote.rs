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
    context: RemoteFusedAttemptContext<'_>,
) -> Result<(u32, AttemptExecutionOutcome)> {
    let RemoteFusedAttemptContext {
        cascade,
        workspace_root,
        options,
        task_run_id,
        placement,
        remote_selection_state,
        remote_workspace,
        workspace_content_hash,
        session,
        execution_label,
        member_execution_labels,
    } = context;
    resolve_attempt_submit_state(
        &cascade.task,
        workspace_root,
        placement,
        AttemptSubmitState {
            remote_workspace,
            workspace_content_hash,
            task_run_id,
            attempt: 1,
            session,
            fused_members: Some(&cascade.members),
            execution_label,
            fused_member_execution_labels: Some(member_execution_labels),
        },
        options.output_observer.as_ref(),
        &options.cancellation,
        remote_selection_state,
    )
    .await?;
    let context = AttemptExecutionContext {
        task: &cascade.task,
        workspace_root,
        run_root: workspace_root,
        placement,
        runtime_metadata: None,
        remote_workspace,
        task_run_id,
        attempt: 1,
        output_observer: options.output_observer.as_ref(),
        cancellation: &options.cancellation,
    };
    execute_task_attempt(&context)
        .await
        .map(|outcome| (1, outcome))
}
