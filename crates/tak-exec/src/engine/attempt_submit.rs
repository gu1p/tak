#[derive(Debug, Clone, Copy)]
struct AttemptSubmitState {
    protocol_mode: RemoteProtocolMode,
    submit_ack: RemoteSubmitAck,
}

async fn preflight_task_placement(task: &ResolvedTask, workspace_root: &Path) -> Result<TaskPlacement> {
    let mut placement = resolve_task_placement(task, workspace_root)?;
    if let Some(target) = &placement.strict_remote_target {
        let mode = preflight_strict_remote_target(target).await?;
        if should_reject_legacy_remote_mode(task, target, mode) {
            bail!("{}", legacy_protocol_error_message(target));
        }
        placement.remote_protocol_mode = Some(mode);
        return Ok(placement);
    }

    if placement.ordered_remote_targets.is_empty() {
        return Ok(placement);
    }

    let (selected, mode) =
        preflight_ordered_remote_target(task, &placement.ordered_remote_targets).await?;
    placement.remote_node_id = Some(selected.node_id.clone());
    placement.strict_remote_target = Some(selected);
    placement.remote_protocol_mode = Some(mode);
    Ok(placement)
}

async fn resolve_initial_runtime_metadata(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    let initial_protocol_mode = placement
        .remote_protocol_mode
        .unwrap_or(RemoteProtocolMode::LegacyReachability);
    if placement.placement_mode == PlacementMode::Remote && initial_protocol_mode.remote_worker() {
        return Ok(None);
    }

    match resolve_runtime_execution_metadata(task, placement) {
        Ok(metadata) => Ok(metadata),
        Err(runtime_error) => {
            if placement.ordered_remote_targets.is_empty()
                || !is_container_lifecycle_failure(&runtime_error)
            {
                return Err(runtime_error);
            }

            let failed_node_id = placement
                .strict_remote_target
                .as_ref()
                .map(|target| target.node_id.clone())
                .ok_or_else(|| {
                    anyhow!(
                        "infra error: missing strict remote target during runtime metadata resolution for task {}",
                        task.label
                    )
                })?;
            let (fallback_target, fallback_mode, fallback_runtime_metadata) =
                fallback_after_container_lifecycle_failure(
                    task,
                    &placement.ordered_remote_targets,
                    &failed_node_id,
                    runtime_error.to_string(),
                )
                .await?;
            placement.remote_node_id = Some(fallback_target.node_id.clone());
            placement.strict_remote_target = Some(fallback_target);
            placement.remote_protocol_mode = Some(fallback_mode);
            Ok(fallback_runtime_metadata)
        }
    }
}

async fn resolve_attempt_submit_state(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
    runtime_metadata: &mut Option<RuntimeExecutionMetadata>,
    remote_workspace: Option<&RemoteWorkspaceStage>,
    task_run_id: &str,
    task_label: &str,
    attempt: u32,
) -> Result<AttemptSubmitState> {
    let mut protocol_mode = placement
        .remote_protocol_mode
        .unwrap_or(RemoteProtocolMode::LegacyReachability);
    let mut submit_ack = RemoteSubmitAck {
        remote_worker: false,
    };

    if placement.placement_mode != PlacementMode::Remote || !protocol_mode.is_handshake_v1() {
        return Ok(AttemptSubmitState {
            protocol_mode,
            submit_ack,
        });
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

    match remote_protocol_submit(
        &target,
        task_run_id,
        attempt,
        task_label,
        task,
        remote_workspace,
        protocol_mode.remote_worker(),
    )
    .await
    {
        Ok(ack) => submit_ack = ack,
        Err(submit_error) => {
            if !placement.ordered_remote_targets.is_empty() && is_auth_submit_failure(&submit_error)
            {
                let failed_node_id = target.node_id.clone();
                let (fallback_target, fallback_mode, fallback_ack) =
                    fallback_after_auth_submit_failure(
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
                    )
                    .await?;
                placement.remote_node_id = Some(fallback_target.node_id.clone());
                placement.strict_remote_target = Some(fallback_target);
                placement.remote_protocol_mode = Some(fallback_mode);
                protocol_mode = fallback_mode;
                if !protocol_mode.remote_worker() {
                    *runtime_metadata = resolve_runtime_execution_metadata(task, placement)?;
                }
                submit_ack = fallback_ack;
            } else {
                return Err(submit_error);
            }
        }
    }

    Ok(AttemptSubmitState {
        protocol_mode,
        submit_ack,
    })
}
