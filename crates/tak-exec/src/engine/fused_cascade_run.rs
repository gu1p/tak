use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use tak_core::model::TaskLabel;
use uuid::Uuid;

use super::attempt_placement::preflight_task_placement;
use super::attempt_submit::resolve_initial_runtime_metadata;
use super::fused_cascade::FusedCascade;
use super::remote_models::TaskPlacement;
use super::remote_selection::SharedRemoteSelectionState;
use super::session_workspaces::SharedExecutionSessionManager;
use super::{LeaseContext, PlacementMode, RunOptions, TaskRunResult};
use crate::lease_client::{TaskLease, acquire_task_lease, release_task_lease};

mod events;
mod local;
mod remote;
mod setup;

struct StartedFusedCascadeContext<'a> {
    cascade: &'a FusedCascade,
    workspace_root: &'a Path,
    options: &'a RunOptions,
    lease_context: &'a LeaseContext,
    sessions: &'a SharedExecutionSessionManager,
    remote_selection_state: &'a SharedRemoteSelectionState,
    task_run_id: &'a str,
    execution_label: Option<&'a str>,
    member_execution_labels: &'a BTreeMap<TaskLabel, String>,
}

pub(crate) struct FusedCascadeRunContext<'a> {
    pub(crate) cascade: &'a FusedCascade,
    pub(crate) workspace_root: &'a Path,
    pub(crate) options: &'a RunOptions,
    pub(crate) lease_context: &'a LeaseContext,
    pub(crate) sessions: &'a SharedExecutionSessionManager,
    pub(crate) remote_selection_state: &'a SharedRemoteSelectionState,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) member_execution_labels: &'a BTreeMap<TaskLabel, String>,
}

pub(crate) async fn run_fused_cascade(
    context: FusedCascadeRunContext<'_>,
) -> Result<TaskRunResult> {
    let FusedCascadeRunContext {
        cascade,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        execution_label,
        member_execution_labels,
    } = context;
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
                remote_selection_state,
            )
            .await?
        }
    };
    events::emit_started(cascade, options, &task_run_id, &placement)?;
    match run_started_fused_cascade(
        StartedFusedCascadeContext {
            cascade,
            workspace_root,
            options,
            lease_context,
            sessions,
            remote_selection_state,
            task_run_id: &task_run_id,
            execution_label,
            member_execution_labels,
        },
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
    context: StartedFusedCascadeContext<'_>,
    placement: &mut TaskPlacement,
) -> Result<TaskRunResult> {
    let StartedFusedCascadeContext {
        cascade,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        task_run_id,
        execution_label,
        member_execution_labels,
    } = context;
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
    let mut lease = acquire_task_lease(&cascade.task, 1, options, lease_context).await?;
    let fused_result = if placement.placement_mode == PlacementMode::Remote {
        remote::run_remote_fused_attempt(remote::RemoteFusedAttemptContext {
            cascade,
            workspace_root,
            options,
            task_run_id,
            placement,
            remote_selection_state,
            remote_workspace: remote_workspace.as_ref(),
            session: prepared_session.as_ref(),
            execution_label,
            member_execution_labels,
        })
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
    release_fused_lease(lease.as_mut(), &cascade.task.label, options).await?;
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

async fn release_fused_lease(
    lease: Option<&mut TaskLease>,
    label: &TaskLabel,
    options: &RunOptions,
) -> Result<()> {
    if let Some(lease) = lease {
        lease.stop_renewal();
        release_task_lease(lease.id(), options)
            .await
            .context(format!("failed releasing lease for {label}"))?;
    }
    Ok(())
}
