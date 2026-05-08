use anyhow::{Context, Result};
use std::path::Path;
use tak_core::model::ResolvedTask;
use uuid::Uuid;

use super::{
    LeaseContext, PlacementMode, RunOptions, TaskRunResult, TaskStartedEvent, TaskStatusPhase,
};

use crate::lease_client::{acquire_task_lease, release_task_lease};
use crate::retry::{retry_backoff_delay, should_retry};
use crate::task_run_metadata::task_run_metadata_for_placement;

use super::attempt_execution::{AttemptExecutionContext, execute_task_attempt};
use super::attempt_placement::preflight_task_placement;
use super::attempt_submit::{
    AttemptSubmitState, resolve_attempt_submit_state, resolve_initial_runtime_metadata,
};
use super::emit_task_started;
use super::output_observer::emit_task_status_message;
use super::remote_models::TaskPlacement;
use super::session_cascade::task_with_session_context;
use super::session_workspaces::ExecutionSessionManager;
use super::task_result::{TaskRunResultContext, build_task_run_result, empty_task_result};
use super::workspace_stage::stage_remote_workspace;

mod events;
use events::emit_finished;

pub(crate) async fn run_single_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &mut ExecutionSessionManager,
    placement_override: Option<TaskPlacement>,
) -> Result<TaskRunResult> {
    if task.steps.is_empty() {
        return Ok(empty_task_result());
    }
    let total_attempts = task.retry.attempts.max(1);
    let mut attempt = 0;
    let task_run_id = Uuid::new_v4().to_string();
    let task_label = task.label.to_string();
    let mut placement = if let Some(placement) = placement_override {
        placement
    } else {
        preflight_task_placement(
            task,
            workspace_root,
            &task_run_id,
            1,
            options.output_observer.as_ref(),
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
    let runtime_metadata = resolve_initial_runtime_metadata(task, &mut placement).await?;
    let remote_stage_task = task_with_session_context(task, placement.session.as_ref());
    let stage_task = remote_stage_task.as_ref().unwrap_or(task);
    let remote_workspace = if placement.placement_mode == PlacementMode::Remote {
        Some(stage_remote_workspace(
            stage_task,
            workspace_root,
            options.output_observer.as_ref(),
        )?)
    } else {
        None
    };
    let prepared_session = sessions.prepare_task(
        task,
        placement.session.as_ref(),
        workspace_root,
        placement.placement_mode == PlacementMode::Local,
    )?;
    let session = prepared_session.as_ref();
    let run_root = if let Some(root) = session.and_then(|session| session.root.as_ref()) {
        root.clone()
    } else if runtime_metadata
        .as_ref()
        .and_then(|metadata| metadata.container_plan.as_ref())
        .is_some()
    {
        workspace_root.to_path_buf()
    } else {
        remote_workspace
            .as_ref()
            .map(|staged| staged.temp_dir.path().to_path_buf())
            .unwrap_or_else(|| workspace_root.to_path_buf())
    };

    loop {
        attempt += 1;

        resolve_attempt_submit_state(
            task,
            &mut placement,
            AttemptSubmitState {
                remote_workspace: remote_workspace.as_ref(),
                task_run_id: &task_run_id,
                task_label: &task_label,
                attempt,
                session,
            },
            options.output_observer.as_ref(),
        )
        .await?;

        let attempt_context = AttemptExecutionContext {
            task,
            workspace_root,
            run_root: &run_root,
            placement: &placement,
            runtime_metadata: runtime_metadata.as_ref(),
            remote_workspace: remote_workspace.as_ref(),
            task_run_id: &task_run_id,
            attempt,
            output_observer: options.output_observer.as_ref(),
        };
        let lease_id = acquire_task_lease(task, attempt, options, lease_context).await?;
        let attempt_result = execute_task_attempt(&attempt_context).await;

        if let Some(id) = lease_id.as_ref() {
            release_task_lease(id, options)
                .await
                .context(format!("failed releasing lease for {}", task.label))?;
        }

        let outcome = attempt_result?;
        if outcome.attempt_success {
            let result = build_task_run_result(
                TaskRunResultContext {
                    task_run_id: &task_run_id,
                    attempt,
                    success: true,
                    placement: &placement,
                    remote_workspace: remote_workspace.as_ref(),
                    runtime_metadata: runtime_metadata.as_ref(),
                    session,
                },
                outcome,
            );
            sessions.finish_task(session, result.success)?;
            emit_finished(options, task, &result)?;
            return Ok(result);
        }

        let can_retry =
            attempt < total_attempts && should_retry(outcome.last_exit_code, &task.retry.on_exit);
        if !can_retry {
            let result = build_task_run_result(
                TaskRunResultContext {
                    task_run_id: &task_run_id,
                    attempt,
                    success: false,
                    placement: &placement,
                    remote_workspace: remote_workspace.as_ref(),
                    runtime_metadata: runtime_metadata.as_ref(),
                    session,
                },
                outcome,
            );
            sessions.finish_task(session, result.success)?;
            emit_finished(options, task, &result)?;
            return Ok(result);
        }

        let wait = retry_backoff_delay(&task.retry.backoff, attempt);
        if placement.placement_mode == PlacementMode::Remote {
            let message = if wait.is_zero() {
                "retrying after failure immediately".to_string()
            } else {
                format!("retrying after failure in {wait:?}")
            };
            emit_task_status_message(
                options.output_observer.as_ref(),
                &task.label,
                attempt + 1,
                TaskStatusPhase::RetryWait,
                placement.remote_node_id.as_deref(),
                message,
            )?;
        }
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}
