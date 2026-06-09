use std::path::{Path, PathBuf};

use anyhow::Result;
use tak_core::model::ResolvedTask;

use super::super::attempt_submit::resolve_initial_runtime_metadata;
use super::super::remote_models::{RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement};
use super::super::remote_selection::SharedRemoteSelectionState;
use super::super::session_cascade::task_with_session_context;
use super::super::session_workspaces::{PreparedTaskSession, SharedExecutionSessionManager};
use super::super::workspace_content_hash::{WorkspaceUploadIdentity, workspace_upload_identity};
use super::super::workspace_stage::stage_remote_workspace;
use super::super::{
    LeaseContext, PlacementMode, RunOptions, TaskFinishedEvent, TaskRunResult, emit_task_finished,
};

mod attempts;
use attempts::{StartedAttemptContext, run_attempts};

pub(super) struct StartedTaskContext<'a> {
    pub(super) task: &'a ResolvedTask,
    pub(super) workspace_root: &'a Path,
    pub(super) options: &'a RunOptions,
    pub(super) lease_context: &'a LeaseContext,
    pub(super) sessions: &'a SharedExecutionSessionManager,
    pub(super) remote_selection_state: &'a SharedRemoteSelectionState,
    pub(super) task_run_id: &'a str,
    pub(super) execution_label: Option<&'a str>,
    pub(super) placement: &'a mut TaskPlacement,
    pub(super) attempt: &'a mut u32,
}

pub(super) async fn run_started_task(context: StartedTaskContext<'_>) -> Result<TaskRunResult> {
    let StartedTaskContext {
        task,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        task_run_id,
        execution_label,
        placement,
        attempt,
    } = context;
    let runtime_metadata = resolve_initial_runtime_metadata(task, placement).await?;
    let (remote_workspace, upload_identity) = stage_remote_workspace_if_needed(
        task,
        workspace_root,
        options,
        placement,
        remote_selection_state,
    )?;
    let prepared_session = sessions.prepare_task(
        task,
        placement.session.as_ref(),
        workspace_root,
        placement.placement_mode == PlacementMode::Local,
    )?;
    let run_root = run_root(
        workspace_root,
        &runtime_metadata,
        &remote_workspace,
        &prepared_session,
    );
    run_attempts(
        task,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        StartedAttemptContext {
            task_run_id,
            execution_label,
            placement,
            attempt,
            runtime_metadata: runtime_metadata.as_ref(),
            remote_workspace: remote_workspace.as_ref(),
            workspace_content_hash: upload_identity.as_ref().map(|id| id.content_hash.as_str()),
            workspace_manifest_hash: upload_identity.as_ref().map(|id| id.manifest_hash.as_str()),
            session: prepared_session.as_ref(),
            run_root: &run_root,
        },
    )
    .await
}

pub(super) fn emit_started_task_failure(
    options: &RunOptions,
    task: &ResolvedTask,
    task_run_id: &str,
    attempts: u32,
    placement: &TaskPlacement,
) -> Result<()> {
    emit_task_finished(
        options.output_observer.as_ref(),
        TaskFinishedEvent {
            task_run_id: task_run_id.to_string(),
            task_label: task.label.clone(),
            attempts: attempts.max(1),
            success: false,
            exit_code: None,
            placement_mode: placement.placement_mode,
            remote_node_id: placement.remote_node_id.clone(),
        },
    )
}

/// Stages the remote workspace and computes its per-job upload-cache content hash — unless the
/// identical content is already uploaded to the chosen node this job, in which case staging is
/// skipped (the staged workspace is `None`) and the later submit references the cached blob.
/// Returns the (possibly absent) stage plus the content hash (absent for non-remote placements).
fn stage_remote_workspace_if_needed(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    placement: &TaskPlacement,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<(
    Option<RemoteWorkspaceStage>,
    Option<WorkspaceUploadIdentity>,
)> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok((None, None));
    }
    let remote_stage_task = task_with_session_context(task, placement.session.as_ref());
    let stage_task = remote_stage_task.as_ref().unwrap_or(task);
    let identity = workspace_upload_identity(workspace_root, &stage_task.context)?;
    if let Some(target) = placement.strict_remote_target.as_ref() {
        let key = (target.node_id.clone(), identity.content_hash.clone());
        if remote_selection_state.upload_cache().peek(&key).is_some() {
            // Already uploaded to this node this job — skip staging entirely.
            return Ok((None, Some(identity)));
        }
    }
    let stage =
        stage_remote_workspace(stage_task, workspace_root, options.output_observer.as_ref())?;
    Ok((Some(stage), Some(identity)))
}

fn run_root(
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
