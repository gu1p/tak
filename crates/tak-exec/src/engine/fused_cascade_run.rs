use std::path::Path;

use anyhow::{Context, Result};
use uuid::Uuid;

use super::attempt_placement::preflight_task_placement;
use super::attempt_submit::resolve_initial_runtime_metadata;
use super::fused_cascade::FusedCascade;
use super::remote_models::TaskPlacement;
use super::session_workspaces::ExecutionSessionManager;
use super::{LeaseContext, PlacementMode, RunOptions, TaskRunResult};
use crate::lease_client::{acquire_task_lease, release_task_lease};

mod events;
mod local;
mod remote;
mod setup;

pub(crate) async fn run_fused_cascade(
    cascade: &FusedCascade,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &mut ExecutionSessionManager,
) -> Result<TaskRunResult> {
    let task_run_id = Uuid::new_v4().to_string();
    let mut placement = match cascade.placement.clone() {
        Some(placement) => placement,
        None => {
            preflight_task_placement(
                &cascade.task,
                workspace_root,
                &task_run_id,
                1,
                options.output_observer.as_ref(),
            )
            .await?
        }
    };
    events::emit_started(cascade, options, &task_run_id, &placement)?;
    match run_started_fused_cascade(
        cascade,
        workspace_root,
        options,
        lease_context,
        sessions,
        &task_run_id,
        &mut placement,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(error) => {
            events::emit_failure(&cascade.task, options, &task_run_id, &placement)?;
            Err(error)
        }
    }
}

async fn run_started_fused_cascade(
    cascade: &FusedCascade,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &mut ExecutionSessionManager,
    task_run_id: &str,
    placement: &mut TaskPlacement,
) -> Result<TaskRunResult> {
    let runtime_metadata = resolve_initial_runtime_metadata(&cascade.task, placement).await?;
    let remote_workspace =
        setup::stage_remote_workspace_if_needed(&cascade.task, workspace_root, options, placement)?;
    let prepared_session = sessions.prepare_task(
        &cascade.task,
        placement.session.as_ref(),
        workspace_root,
        placement.placement_mode == PlacementMode::Local,
    )?;
    let run_root = setup::run_root(
        workspace_root,
        &runtime_metadata,
        &remote_workspace,
        &prepared_session,
    );
    let lease_id = acquire_task_lease(&cascade.task, 1, options, lease_context).await?;
    let fused_result = if placement.placement_mode == PlacementMode::Remote {
        remote::run_remote_fused_attempt(
            cascade,
            workspace_root,
            options,
            task_run_id,
            placement,
            remote_workspace.as_ref(),
            prepared_session.as_ref(),
        )
        .await
    } else {
        local::run_local_fused_attempt(local::LocalFusedAttemptContext {
            cascade,
            workspace_root,
            options,
            task_run_id,
            placement,
            runtime_metadata: runtime_metadata.as_ref(),
            remote_workspace: remote_workspace.as_ref(),
            run_root: &run_root,
        })
        .await
    };
    if let Some(id) = lease_id.as_ref() {
        release_task_lease(id, options)
            .await
            .context(format!("failed releasing lease for {}", cascade.task.label))?;
    }
    let (attempts, outcome) = fused_result?;
    let result = setup::build_task_result(
        task_run_id,
        attempts,
        outcome,
        placement,
        remote_workspace.as_ref(),
        runtime_metadata.as_ref(),
        prepared_session.as_ref(),
    );
    sessions.finish_task(prepared_session.as_ref(), result.success)?;
    events::emit_finished(&cascade.task, options, &result)?;
    Ok(result)
}
