use std::path::Path;

use anyhow::{Result, anyhow};
use tak_core::model::ResolvedTask;

use crate::step_runner::StepRunResult;

use super::output_observer::emit_task_status_message;
use super::protocol_cancel::remote_protocol_cancel;
use super::protocol_events::remote_protocol_events;
use super::protocol_result_http::remote_protocol_result;
use super::remote_models::{RemoteWorkspaceStage, RuntimeExecutionMetadata, TaskPlacement};
use super::result_tail_recovery::recover_missing_remote_result_tails;
use super::step_execution::run_task_steps_with_runtime;
use super::workspace_outputs::collect_workspace_outputs;
use super::{
    PlacementMode, RemoteLogChunk, RunCancellation, SyncedOutput, TaskOutputObserver,
    TaskStatusPhase,
};

mod sync_outputs;
use sync_outputs::sync_attempt_outputs;

pub(crate) struct AttemptExecutionContext<'a> {
    pub(crate) task: &'a ResolvedTask,
    pub(crate) workspace_root: &'a Path,
    pub(crate) run_root: &'a Path,
    pub(crate) placement: &'a TaskPlacement,
    pub(crate) runtime_metadata: Option<&'a RuntimeExecutionMetadata>,
    pub(crate) remote_workspace: Option<&'a RemoteWorkspaceStage>,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) output_observer: Option<&'a std::sync::Arc<dyn TaskOutputObserver>>,
    pub(crate) cancellation: &'a RunCancellation,
}

pub(crate) struct AttemptExecutionOutcome {
    pub(crate) attempt_success: bool,
    pub(crate) last_exit_code: Option<i32>,
    pub(crate) failure_detail: Option<String>,
    pub(crate) synced_outputs: Vec<SyncedOutput>,
    pub(crate) remote_runtime_kind: Option<String>,
    pub(crate) remote_runtime_engine: Option<String>,
    pub(crate) remote_logs: Vec<RemoteLogChunk>,
}

pub(crate) async fn execute_task_attempt(
    context: &AttemptExecutionContext<'_>,
) -> Result<AttemptExecutionOutcome> {
    let run_local_attempt = context.placement.placement_mode != PlacementMode::Remote;
    let run_result = if run_local_attempt {
        run_task_steps_with_runtime(
            context.task,
            context.run_root,
            context.runtime_metadata,
            context.attempt,
            context.task_run_id,
            context.output_observer,
            context.cancellation,
        )
        .await
    } else {
        Ok(StepRunResult {
            success: true,
            exit_code: Some(0),
        })
    };

    let (remote_logs, protocol_result) = if context.placement.placement_mode
        == PlacementMode::Remote
    {
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
        let events = remote_protocol_events(
            target,
            context.task_run_id,
            &context.task.label,
            context.attempt,
            context.output_observer,
        );
        tokio::pin!(events);
        let (mut remote_logs, protocol_result) = tokio::select! {
            result = &mut events => result?,
            _ = context.cancellation.cancelled() => {
                let _ = remote_protocol_cancel(target, context.task_run_id, context.attempt).await;
                return Err(super::cancellation::cancelled_error());
            }
        };
        let result = match protocol_result {
            Some(result) => result,
            None => remote_protocol_result(target, context.task_run_id, context.attempt).await?,
        };
        recover_missing_remote_result_tails(
            context.task_run_id,
            &context.task.label,
            context.attempt,
            context.output_observer,
            &mut remote_logs,
            &result,
        )?;
        (remote_logs, Some(result))
    } else {
        (Vec::new(), None)
    };

    let run = run_result?;
    let has_remote_protocol_result = protocol_result.is_some();
    let (
        attempt_success,
        last_exit_code,
        failure_detail,
        mut synced_outputs,
        remote_runtime_kind,
        remote_runtime_engine,
    ) = match protocol_result {
        Some(remote_result) => (
            remote_result.success,
            remote_result.exit_code.or(run.exit_code),
            remote_result.failure_detail,
            remote_result.synced_outputs,
            remote_result.runtime_kind,
            remote_result.runtime_engine,
        ),
        None => (run.success, run.exit_code, None, Vec::new(), None, None),
    };

    if !has_remote_protocol_result
        && run_local_attempt
        && context.run_root != context.workspace_root
    {
        synced_outputs =
            collect_workspace_outputs(context.run_root, &context.task.outputs, attempt_success)?;
    }

    if !synced_outputs.is_empty() {
        if context.placement.placement_mode == PlacementMode::Remote {
            emit_task_status_message(
                context.output_observer,
                &context.task.label,
                context.attempt,
                TaskStatusPhase::RemoteSyncOutputs,
                context.placement.remote_node_id.as_deref(),
                format!("syncing remote outputs ({} files)", synced_outputs.len()),
            )?;
        }
        sync_attempt_outputs(context, &synced_outputs, run_local_attempt).await?;
    }

    Ok(AttemptExecutionOutcome {
        attempt_success,
        last_exit_code,
        failure_detail,
        synced_outputs,
        remote_runtime_kind,
        remote_runtime_engine,
        remote_logs,
    })
}
