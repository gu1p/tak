use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use tak_core::model::{ResolvedTask, TaskLabel};

use super::{PlacementMode, TaskOutputObserver, TaskStatusPhase};

use super::output_observer::emit_task_status_message;
use super::preflight_fallback::{
    fallback_after_auth_submit_failure, is_auth_submit_failure, preflight_ordered_remote_target,
};
use super::protocol_submit::{RemoteProtocolSubmit, remote_protocol_submit};
use super::remote_models::{
    RemoteSubmitContext, RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement,
};
use super::remote_selection::SharedRemoteSelectionState;
use super::runtime_metadata::resolve_runtime_execution_metadata;
use super::session_workspaces::PreparedTaskSession;

pub(crate) struct AttemptSubmitState<'a> {
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
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
    let remote_workspace = submit.remote_workspace.ok_or_else(|| {
        anyhow!(
            "infra error: missing staged remote workspace during submit for task {}",
            task.label
        )
    })?;
    emit_task_status_message(
        output_observer,
        &task.label,
        submit.attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(target.node_id.as_str()),
        format!(
            "submitting {} to remote node {}",
            remote_workspace.upload_size_mb(),
            target.node_id
        ),
    )?;

    match remote_protocol_submit(RemoteProtocolSubmit {
        target: &target,
        task_run_id: submit.task_run_id,
        attempt: submit.attempt,
        task,
        remote_workspace,
        session: submit.session,
        fused_members: submit.fused_members,
        execution_label: submit.execution_label,
        fused_member_execution_labels: submit.fused_member_execution_labels,
    })
    .await
    {
        Ok(()) => {
            emit_task_status_message(
                output_observer,
                &task.label,
                submit.attempt,
                TaskStatusPhase::RemoteSubmit,
                Some(target.node_id.as_str()),
                format!("remote task accepted by {}", target.node_id),
            )?;
        }
        Err(submit_error) => {
            let submit_error = anyhow::Error::new(submit_error);
            if !placement.ordered_remote_targets.is_empty() && is_auth_submit_failure(&submit_error)
            {
                let failed_node_id = target.node_id.clone();
                let fallback_target = fallback_after_auth_submit_failure(
                    task,
                    &placement.ordered_remote_targets,
                    &failed_node_id,
                    RemoteSubmitContext {
                        task_run_id: submit.task_run_id,
                        attempt: submit.attempt,
                        remote_workspace,
                        session: submit.session,
                        fused_members: submit.fused_members,
                        execution_label: submit.execution_label,
                        fused_member_execution_labels: submit.fused_member_execution_labels,
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

async fn refresh_remote_target_for_attempt(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
    task_run_id: &str,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<()> {
    if attempt == 1 && placement.strict_remote_target.is_some() {
        return Ok(());
    }
    let ordered = remote_selection_state.reserve_ordered_targets_for_attempt(
        &placement.ordered_remote_targets,
        placement.remote_selection,
        &task.label.to_string(),
        task_run_id,
        attempt,
    );
    let reserved_node_id = ordered.first().map(|target| target.node_id.clone());
    let selected = match preflight_ordered_remote_target(
        task,
        &ordered,
        placement.remote_selection,
        output_observer,
    )
    .await
    {
        Ok(selected) => selected,
        Err(err) => {
            remote_selection_state
                .release_reserved_target(placement.remote_selection, reserved_node_id.as_deref());
            return Err(err);
        }
    };
    remote_selection_state.confirm_selected_target(
        placement.remote_selection,
        reserved_node_id.as_deref(),
        &selected.node_id,
    );
    placement.ordered_remote_targets = ordered;
    placement.remote_node_id = Some(selected.node_id.clone());
    placement.strict_remote_target = Some(selected);
    Ok(())
}
