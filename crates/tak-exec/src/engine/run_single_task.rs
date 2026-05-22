use anyhow::{Context, Result};
use std::path::Path;
use tak_core::model::ResolvedTask;
use uuid::Uuid;

use crate::task_run_metadata::task_run_metadata_for_placement;

use super::attempt_placement::preflight_task_placement;
use super::emit_task_started;
use super::remote_models::TaskPlacement;
use super::remote_selection::SharedRemoteSelectionState;
use super::session_workspaces::SharedExecutionSessionManager;
use super::task_result::empty_task_result;
use super::{LeaseContext, RunOptions, TaskRunResult, TaskStartedEvent};

mod events;
mod started_task;
use started_task::{StartedTaskContext, emit_started_task_failure, run_started_task};

pub(crate) struct RunSingleTaskContext<'a> {
    pub(crate) task: &'a ResolvedTask,
    pub(crate) workspace_root: &'a Path,
    pub(crate) options: &'a RunOptions,
    pub(crate) lease_context: &'a LeaseContext,
    pub(crate) sessions: &'a SharedExecutionSessionManager,
    pub(crate) remote_selection_state: &'a SharedRemoteSelectionState,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) placement_override: Option<TaskPlacement>,
}

pub(crate) async fn run_single_task(context: RunSingleTaskContext<'_>) -> Result<TaskRunResult> {
    let RunSingleTaskContext {
        task,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        execution_label,
        placement_override,
    } = context;
    if task.steps.is_empty() {
        return Ok(empty_task_result());
    }
    let task_run_id = Uuid::new_v4().to_string();
    let mut placement = if let Some(placement) = placement_override {
        placement
    } else {
        preflight_task_placement(
            task,
            workspace_root,
            &task_run_id,
            1,
            options.output_observer.as_ref(),
            remote_selection_state,
        )
        .await?
    };
    let metadata = task_run_metadata_for_placement(task, &placement);
    emit_task_started(
        options.output_observer.as_ref(),
        TaskStartedEvent {
            task_run_id: task_run_id.clone(),
            task_label: task.label.clone(),
            placement_mode: placement.placement_mode,
            remote_node_id: placement.remote_node_id.clone(),
            origin: Some(metadata.origin),
            runtime: metadata.runtime,
            runtime_source: metadata.runtime_source,
            command: metadata.command,
        },
    )?;
    let mut attempt = 0;
    match run_started_task(StartedTaskContext {
        task,
        workspace_root,
        options,
        lease_context,
        sessions,
        remote_selection_state,
        task_run_id: &task_run_id,
        execution_label,
        placement: &mut placement,
        attempt: &mut attempt,
    })
    .await
    {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Err(finish_error) =
                emit_started_task_failure(options, task, &task_run_id, attempt, &placement)
            {
                return Err(error)
                    .context(format!("failed to record task failure: {finish_error}"));
            }
            Err(error)
        }
    }
}
