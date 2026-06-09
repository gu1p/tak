use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, anyhow};
use tak_core::model::{ResolvedTask, TaskLabel};

use super::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use super::preflight_fallback::{fallback_after_auth_submit_failure, is_auth_submit_failure};
use super::protocol_submit::{RemoteProtocolSubmit, remote_protocol_submit};
use super::remote_models::{
    RemoteSubmitContext, RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement,
};
use super::remote_selection::SharedRemoteSelectionState;
use super::runtime_metadata::resolve_runtime_execution_metadata;
use super::session_cascade::task_with_session_context;
use super::session_workspaces::PreparedTaskSession;
use super::workspace_stage::stage_remote_workspace;
use super::{PlacementMode, TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

mod target_refresh;
mod upload_progress;
use target_refresh::refresh_remote_target_for_attempt;

pub(crate) struct AttemptSubmitState<'a> {
    /// The eagerly-staged workspace (miss path). `None` on the cache-hit path, where staging
    /// was skipped up front; it is staged lazily here only if an upload turns out to be needed.
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    /// Deterministic content hash of the staged workspace, keying the per-job upload cache.
    /// `None` for non-remote placements.
    pub(crate) workspace_content_hash: Option<&'a str>,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) session: Option<&'a PreparedTaskSession>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
}

pub(crate) async fn resolve_initial_runtime_metadata(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    if placement.placement_mode == PlacementMode::Remote {
        return Ok(None);
    }
    resolve_runtime_execution_metadata(task, placement)
}

pub(crate) async fn resolve_attempt_submit_state(
    task: &ResolvedTask,
    workspace_root: &Path,
    placement: &mut TaskPlacement,
    submit: AttemptSubmitState<'_>,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    cancellation: &super::RunCancellation,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<()> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(());
    }
    if cancellation.is_cancelled() {
        return Err(super::cancelled_error());
    }
    refresh_remote_target_for_attempt(
        task,
        placement,
        submit.task_run_id,
        submit.attempt,
        output_observer,
        remote_selection_state,
    )
    .await?;

    let target = placement.strict_remote_target.clone().ok_or_else(|| {
        anyhow!(
            "infra error: missing strict remote target during submit for task {}",
            task.label
        )
    })?;
    let content_hash = submit.workspace_content_hash.ok_or_else(|| {
        anyhow!(
            "infra error: missing workspace content hash during submit for task {}",
            task.label
        )
    })?;
    let upload_cache = remote_selection_state.upload_cache();
    let session_stage_task = task_with_session_context(task, placement.session.as_ref());
    let stage_task = session_stage_task.as_ref().unwrap_or(task);

    // Owns a workspace staged lazily on the cache-hit path (when run_started_task skipped it
    // because the blob was already cached, but the blob then turned out to be missing or this
    // task became the single-flight leader).
    let mut staged_owned: Option<RemoteWorkspaceStage> = None;

    // Show the upload progress bracket only when we actually hold a staged archive to send;
    // on a pure cache hit nothing is transferred.
    let upload_progress = submit
        .remote_workspace
        .map(|stage| {
            upload_progress::start_upload_progress(
                output_observer,
                &task.label,
                submit.attempt,
                &target,
                stage,
            )
        })
        .transpose()?;

    // First submit attempt with whatever stage we have (None on the cache-hit path).
    let first = remote_protocol_submit(RemoteProtocolSubmit {
        target: &target,
        task_run_id: submit.task_run_id,
        attempt: submit.attempt,
        task,
        remote_workspace: submit.remote_workspace,
        session: submit.session,
        fused_members: submit.fused_members,
        execution_label: submit.execution_label,
        fused_member_execution_labels: submit.fused_member_execution_labels,
        output_observer,
        upload_cache,
        workspace_content_hash: content_hash,
    })
    .await;

    // On the cache-hit path the referenced blob may be gone (or we became the leader). Stage
    // the workspace now and retry once with a real upload.
    let submit_outcome = match first {
        Err(err) if err.is_missing_upload() && submit.remote_workspace.is_none() => {
            staged_owned = Some(stage_remote_workspace(
                stage_task,
                workspace_root,
                output_observer,
            )?);
            remote_protocol_submit(RemoteProtocolSubmit {
                target: &target,
                task_run_id: submit.task_run_id,
                attempt: submit.attempt,
                task,
                remote_workspace: staged_owned.as_ref(),
                session: submit.session,
                fused_members: submit.fused_members,
                execution_label: submit.execution_label,
                fused_member_execution_labels: submit.fused_member_execution_labels,
                output_observer,
                upload_cache,
                workspace_content_hash: content_hash,
            })
            .await
        }
        other => other,
    };

    match submit_outcome {
        Ok(selected_target) => {
            if let (Some(progress), Some(stage)) = (upload_progress, submit.remote_workspace) {
                upload_progress::finish_upload_progress(
                    output_observer,
                    &task.label,
                    submit.attempt,
                    &selected_target,
                    stage,
                    progress,
                )?;
            }
            placement.remote_node_id = Some(selected_target.node_id.clone());
            placement.strict_remote_target = Some(selected_target.clone());
            let accepted_message = if selected_target.daemon_task_handle.is_some() {
                format!(
                    "remote worker {} selected by local takd; task accepted",
                    selected_target.node_id
                )
            } else {
                format!("remote task accepted by {}", selected_target.node_id)
            };
            emit_task_status_message_with_details(
                output_observer,
                &task.label,
                submit.attempt,
                TaskStatusPhase::RemoteSubmit,
                Some(selected_target.node_id.as_str()),
                accepted_message,
                TaskStatusDetails {
                    kind: Some(TaskStatusEventKind::WorkerSelected),
                    transport: Some(selected_target.transport_kind.as_result_value().to_string()),
                    ..TaskStatusDetails::default()
                },
            )?;
        }
        Err(submit_error) => {
            let submit_error = anyhow::Error::new(submit_error);
            if !placement.ordered_remote_targets.is_empty() && is_auth_submit_failure(&submit_error)
            {
                // The auth fallback re-uploads to other candidates, so it needs a real staged
                // workspace; stage now if the cache-hit path skipped it.
                if submit.remote_workspace.is_none() && staged_owned.is_none() {
                    staged_owned = Some(stage_remote_workspace(
                        stage_task,
                        workspace_root,
                        output_observer,
                    )?);
                }
                let fallback_stage = submit
                    .remote_workspace
                    .or(staged_owned.as_ref())
                    .expect("staged workspace available for auth fallback");
                let failed_node_id = target.node_id.clone();
                let fallback_target = fallback_after_auth_submit_failure(
                    task,
                    &placement.ordered_remote_targets,
                    &failed_node_id,
                    RemoteSubmitContext {
                        task_run_id: submit.task_run_id,
                        attempt: submit.attempt,
                        remote_workspace: fallback_stage,
                        session: submit.session,
                        fused_members: submit.fused_members,
                        execution_label: submit.execution_label,
                        fused_member_execution_labels: submit.fused_member_execution_labels,
                        upload_cache,
                        workspace_content_hash: content_hash,
                    },
                    submit_error.to_string(),
                    output_observer,
                )
                .await?;
                remote_selection_state.replace_assignment(
                    placement.remote_selection,
                    &failed_node_id,
                    &fallback_target.node_id,
                );
                placement.remote_node_id = Some(fallback_target.node_id.clone());
                placement.strict_remote_target = Some(fallback_target);
            } else {
                return Err(submit_error);
            }
        }
    }

    Ok(())
}
