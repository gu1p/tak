use std::path::Path;

use anyhow::{Result, anyhow};
use tak_core::model::ResolvedTask;

use super::{AttemptSubmitState, acceptance, target_refresh, upload_progress};
use crate::engine::preflight_fallback::{
    fallback_after_auth_submit_failure, is_auth_submit_failure,
};
use crate::engine::protocol_submit::{RemoteProtocolSubmit, remote_protocol_submit};
use crate::engine::remote_models::{RemoteSubmitContext, RemoteWorkspaceStage, TaskPlacement};
use crate::engine::remote_selection::SharedRemoteSelectionState;
use crate::engine::session_cascade::task_with_session_context;
use crate::engine::workspace_stage::stage_remote_workspace;
use crate::engine::{PlacementMode, TaskOutputObserver};

pub(crate) async fn resolve_attempt_submit_state(
    task: &ResolvedTask,
    workspace_root: &Path,
    placement: &mut TaskPlacement,
    submit: AttemptSubmitState<'_>,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    cancellation: &crate::engine::RunCancellation,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<()> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(());
    }
    if cancellation.is_cancelled() {
        return Err(crate::engine::cancelled_error());
    }
    target_refresh::refresh_remote_target_for_attempt(
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
    let mut staged_owned: Option<RemoteWorkspaceStage> = None;
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
            acceptance::record_accepted_target(
                task,
                placement,
                &submit,
                output_observer,
                selected_target,
            )?;
        }
        Err(submit_error) => {
            if let Some(failed_node_id) = submit_error.failed_node_id.as_ref() {
                placement.remote_node_id = Some(failed_node_id.clone());
            }
            let submit_error = anyhow::Error::new(submit_error);
            if placement.ordered_remote_targets.is_empty() || !is_auth_submit_failure(&submit_error)
            {
                return Err(submit_error);
            }
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
                &mut placement.infrastructure_failures,
                output_observer,
            )
            .await?;
            remote_selection_state.replace_assignment(
                placement.remote_selection,
                &failed_node_id,
                &fallback_target.node_id,
            );
            placement.remote_node_id = Some(fallback_target.node_id.clone());
            let mut fallback_target = fallback_target;
            fallback_target.excluded_node_ids = placement
                .infrastructure_failures
                .iter()
                .map(|failure| failure.node_id.clone())
                .collect();
            placement.strict_remote_target = Some(fallback_target);
        }
    }
    Ok(())
}
