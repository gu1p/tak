use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::attempt_execution::AttemptExecutionOutcome;
use crate::engine::remote_models::{RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement};
use crate::engine::session_cascade::task_with_session_context;
use crate::engine::session_workspaces::PreparedTaskSession;
use crate::engine::task_result::{TaskRunResultContext, build_task_run_result};
use crate::engine::workspace_stage::stage_remote_workspace;
use crate::engine::{PlacementMode, RunOptions, TaskRunResult};

pub(super) fn build_task_result(
    task_run_id: &str,
    attempts: u32,
    outcome: AttemptExecutionOutcome,
    placement: &TaskPlacement,
    remote_workspace: Option<&RemoteWorkspaceStage>,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
    session: Option<&PreparedTaskSession>,
) -> TaskRunResult {
    let success = outcome.attempt_success;
    build_task_run_result(
        TaskRunResultContext {
            task_run_id,
            attempt: attempts,
            success,
            placement,
            remote_workspace,
            runtime_metadata,
            session,
        },
        outcome,
    )
}

pub(super) fn stage_remote_workspace_if_needed(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    placement: &TaskPlacement,
) -> Result<Option<RemoteWorkspaceStage>> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(None);
    }
    let remote_stage_task = task_with_session_context(task, placement.session.as_ref());
    let stage_task = remote_stage_task.as_ref().unwrap_or(task);
    Ok(Some(stage_remote_workspace(
        stage_task,
        workspace_root,
        options.output_observer.as_ref(),
    )?))
}

pub(super) fn run_root(
    workspace_root: &Path,
    runtime_metadata: &Option<RuntimeExecutionMetadata>,
    remote_workspace: &Option<RemoteWorkspaceStage>,
    prepared_session: &Option<PreparedTaskSession>,
) -> PathBuf {
    if let Some(root) = prepared_session
        .as_ref()
        .and_then(|session| session.root.as_ref())
    {
        return root.clone();
    }
    if runtime_metadata
        .as_ref()
        .and_then(|metadata| metadata.container_plan.as_ref())
        .is_some()
    {
        return workspace_root.to_path_buf();
    }
    remote_workspace
        .as_ref()
        .map(|staged| staged.temp_dir.path().to_path_buf())
        .unwrap_or_else(|| workspace_root.to_path_buf())
}
