struct AttemptExecutionContext<'a> {
    task: &'a ResolvedTask,
    workspace_root: &'a Path,
    run_root: &'a Path,
    placement: &'a TaskPlacement,
    runtime_metadata: Option<&'a RuntimeExecutionMetadata>,
    remote_workspace: Option<&'a RemoteWorkspaceStage>,
    task_run_id: &'a str,
    attempt: u32,
}

struct AttemptExecutionOutcome {
    attempt_success: bool,
    last_exit_code: Option<i32>,
    synced_outputs: Vec<SyncedOutput>,
    remote_runtime_kind: Option<String>,
    remote_runtime_engine: Option<String>,
    remote_logs: Vec<RemoteLogChunk>,
}

async fn execute_task_attempt(
    context: &AttemptExecutionContext<'_>,
) -> Result<AttemptExecutionOutcome> {
    let run_local_attempt = context.placement.placement_mode != PlacementMode::Remote;
    let run_result = if run_local_attempt {
        run_task_steps_with_runtime(context.task, context.run_root, context.runtime_metadata).await
    } else {
        Ok(StepRunResult {
            success: true,
            exit_code: Some(0),
        })
    };

    let (remote_logs, protocol_result) = if context.placement.placement_mode == PlacementMode::Remote {
        let target = context
            .placement
            .strict_remote_target
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "infra error: missing strict remote target during events/result for task {}",
                    context.task.label
                )
            })?;
        let remote_logs = remote_protocol_events(target, context.task_run_id).await?;
        let result = remote_protocol_result(target, context.task_run_id, context.attempt).await?;
        (remote_logs, Some(result))
    } else {
        (Vec::new(), None)
    };

    let run = run_result?;
    let (
        attempt_success,
        last_exit_code,
        synced_outputs,
        remote_runtime_kind,
        remote_runtime_engine,
    ) = match protocol_result {
        Some(remote_result) => (
            remote_result.success,
            remote_result.exit_code.or(run.exit_code),
            remote_result.synced_outputs,
            remote_result.runtime_kind,
            remote_result.runtime_engine,
        ),
        None => (run.success, run.exit_code, Vec::new(), None, None),
    };

    if !synced_outputs.is_empty() {
        sync_attempt_outputs(context, &synced_outputs, run_local_attempt).await?;
    }

    Ok(AttemptExecutionOutcome {
        attempt_success,
        last_exit_code,
        synced_outputs,
        remote_runtime_kind,
        remote_runtime_engine,
        remote_logs,
    })
}

async fn sync_attempt_outputs(
    context: &AttemptExecutionContext<'_>,
    synced_outputs: &[SyncedOutput],
    run_local_attempt: bool,
) -> Result<()> {
    if run_local_attempt {
        if let Some(staged_workspace) = context.remote_workspace {
            sync_remote_outputs(
                staged_workspace.temp_dir.path(),
                context.workspace_root,
                synced_outputs,
            )?;
        }
        return Ok(());
    }

    let target = context
        .placement
        .strict_remote_target
        .as_ref()
        .ok_or_else(|| {
            anyhow!(
                "infra error: missing strict remote target during output sync for task {}",
                context.task.label
            )
        })?;
    sync_remote_outputs_from_remote(
        target,
        context.task_run_id,
        context.attempt,
        context.workspace_root,
        synced_outputs,
    )
    .await
}
