use anyhow::{Result, anyhow};
use tak_core::model::ResolvedTask;

use super::{PlacementMode, TaskOutputObserver, TaskStatusPhase};

use super::output_observer::emit_task_status_message;
use super::preflight_fallback::{
    fallback_after_auth_submit_failure, is_auth_submit_failure, preflight_ordered_remote_target,
};
use super::protocol_submit::remote_protocol_submit;
use super::remote_models::{
    RemoteSubmitContext, RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement,
};
use super::remote_selection::ordered_remote_targets_for_attempt;
use super::runtime_metadata::resolve_runtime_execution_metadata;
use super::session_workspaces::PreparedTaskSession;

pub(crate) struct AttemptSubmitState<'a> {
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(crate) task_run_id: &'a str,
    pub(crate) task_label: &'a str,
    pub(crate) attempt: u32,
    pub(crate) session: Option<&'a PreparedTaskSession>,
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
) -> Result<()> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(());
    }
    refresh_remote_target_for_attempt(
        task,
        placement,
        submit.task_run_id,
        submit.attempt,
        output_observer,
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
        format!("submitting to remote node {}", target.node_id),
    )?;

    match remote_protocol_submit(
        &target,
        submit.task_run_id,
        submit.attempt,
        submit.task_label,
        task,
        remote_workspace,
        submit.session,
    )
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
                        task_label: submit.task_label,
                        remote_workspace,
                        session: submit.session,
                    },
                    submit_error.to_string(),
                    output_observer,
                )
                .await?;
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
) -> Result<()> {
    if attempt == 1 && placement.strict_remote_target.is_some() {
        return Ok(());
    }
    let ordered = ordered_remote_targets_for_attempt(
        &placement.ordered_remote_targets,
        placement.remote_selection,
        &task.label.to_string(),
        task_run_id,
        attempt,
    );
    let selected = preflight_ordered_remote_target(task, &ordered, output_observer).await?;
    placement.ordered_remote_targets = ordered;
    placement.remote_node_id = Some(selected.node_id.clone());
    placement.strict_remote_target = Some(selected);
    Ok(())
}
