use std::path::Path;

use anyhow::Result;

use crate::engine::RunOptions;
use crate::engine::attempt_execution::{
    AttemptExecutionContext, AttemptExecutionOutcome, execute_task_attempt,
};
use crate::engine::attempt_submit::{AttemptSubmitState, resolve_attempt_submit_state};
use crate::engine::fused_cascade::FusedCascade;
use crate::engine::remote_models::{RemoteWorkspaceStage, TaskPlacement};
use crate::engine::session_workspaces::PreparedTaskSession;

pub(super) async fn run_remote_fused_attempt(
    cascade: &FusedCascade,
    workspace_root: &Path,
    options: &RunOptions,
    task_run_id: &str,
    placement: &mut TaskPlacement,
    remote_workspace: Option<&RemoteWorkspaceStage>,
    session: Option<&PreparedTaskSession>,
) -> Result<(u32, AttemptExecutionOutcome)> {
    resolve_attempt_submit_state(
        &cascade.task,
        placement,
        AttemptSubmitState {
            remote_workspace,
            task_run_id,
            attempt: 1,
            session,
            fused_members: Some(&cascade.members),
        },
        options.output_observer.as_ref(),
        &options.cancellation,
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
