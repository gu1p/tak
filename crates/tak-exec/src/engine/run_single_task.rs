use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use tak_core::model::ResolvedTask;
use uuid::Uuid;

use super::{LeaseContext, PlacementMode, RunOptions, TaskRunResult, TaskStatusPhase};

use crate::lease_client::{acquire_task_lease, release_task_lease};
use crate::retry::{retry_backoff_delay, should_retry};

use super::attempt_execution::{AttemptExecutionContext, execute_task_attempt};
use super::attempt_submit::{
    AttemptSubmitState, preflight_task_placement, resolve_attempt_submit_state,
    resolve_initial_runtime_metadata,
};
use super::output_observer::emit_task_status_message;
use super::remote_models::TaskPlacement;
use super::session_cascade::task_with_session_context;
use super::session_workspaces::PreparedTaskSession;
use super::task_result::build_task_run_result;
use super::workspace_stage::stage_remote_workspace;

/// Runs one task with retries, acquiring and releasing leases per attempt when configured.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn run_single_task(
    task: &ResolvedTask,
    workspace_root: &Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    session: Option<&PreparedTaskSession>,
) -> Result<TaskRunResult> {
    if task.steps.is_empty() {
        return Ok(empty_task_result());
    }
    let mut placement =
        preflight_task_placement(task, workspace_root, options.output_observer.as_ref()).await?;
    let runtime_metadata = resolve_initial_runtime_metadata(task, &mut placement).await?;
    let remote_stage_task = task_with_session_context(task);
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

    let total_attempts = task.retry.attempts.max(1);
    let mut attempt = 0;
    let task_run_id = Uuid::new_v4().to_string();
    let task_label = task.label.to_string();

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
            return Ok(build_task_run_result(
                attempt,
                true,
                &placement,
                remote_workspace.as_ref(),
                runtime_metadata.as_ref(),
                session,
                outcome,
            ));
        }

        let can_retry =
            attempt < total_attempts && should_retry(outcome.last_exit_code, &task.retry.on_exit);
        if !can_retry {
            return Ok(build_task_run_result(
                attempt,
                false,
                &placement,
                remote_workspace.as_ref(),
                runtime_metadata.as_ref(),
                session,
                outcome,
            ));
        }

        let wait = retry_backoff_delay(&task.retry.backoff, attempt);
        if placement.placement_mode == PlacementMode::Remote {
            let message = if wait.is_zero() {
                "retrying after failure immediately".to_string()
            } else {
                format!("retrying after failure in {}", format_status_duration(wait))
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

fn empty_task_result() -> TaskRunResult {
    let placement = TaskPlacement {
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: Vec::new(),
        decision_reason: None,
        local: None,
    };
    build_task_run_result(
        1,
        true,
        &placement,
        None,
        None,
        None,
        super::attempt_execution::AttemptExecutionOutcome {
            attempt_success: true,
            last_exit_code: Some(0),
            failure_detail: None,
            synced_outputs: Vec::new(),
            remote_runtime_kind: None,
            remote_runtime_engine: None,
            remote_logs: Vec::new(),
        },
    )
}

fn format_status_duration(duration: Duration) -> String {
    let whole_seconds = duration.as_secs();
    if duration.subsec_nanos() == 0 {
        format!("{whole_seconds}s")
    } else {
        format!("{:.1}s", duration.as_secs_f64())
    }
}
