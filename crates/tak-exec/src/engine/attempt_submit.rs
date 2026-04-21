use super::*;

pub(crate) async fn preflight_task_placement(
    task: &ResolvedTask,
    workspace_root: &Path,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<TaskPlacement> {
    let mut placement = resolve_task_placement(task, workspace_root)?;
    if placement.ordered_remote_targets.is_empty() {
        return Ok(placement);
    }

    let selected =
        preflight_ordered_remote_target(task, &placement.ordered_remote_targets, output_observer)
            .await?;
    placement.remote_node_id = Some(selected.node_id.clone());
    placement.strict_remote_target = Some(selected);
    Ok(placement)
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
    remote_workspace: Option<&RemoteWorkspaceStage>,
    task_run_id: &str,
    task_label: &str,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<()> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(());
    }

    let target = placement.strict_remote_target.clone().ok_or_else(|| {
        anyhow!(
            "infra error: missing strict remote target during submit for task {}",
            task.label
        )
    })?;
    let remote_workspace = remote_workspace.ok_or_else(|| {
        anyhow!(
            "infra error: missing staged remote workspace during submit for task {}",
            task.label
        )
    })?;
    emit_task_status_message(
        output_observer,
        &task.label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(target.node_id.as_str()),
        format!("submitting to remote node {}", target.node_id),
    )?;

    match remote_protocol_submit(
        &target,
        task_run_id,
        attempt,
        task_label,
        task,
        remote_workspace,
    )
    .await
    {
        Ok(()) => {
            emit_task_status_message(
                output_observer,
                &task.label,
                attempt,
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
                        task_run_id,
                        attempt,
                        task_label,
                        remote_workspace,
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
