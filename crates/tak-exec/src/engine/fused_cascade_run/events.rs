use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::fused_cascade::FusedCascade;
use crate::engine::remote_models::TaskPlacement;
use crate::engine::{
    RunOptions, TaskFinishedEvent, TaskRunResult, TaskStartedEvent, emit_task_finished,
    emit_task_started,
};

pub(super) fn emit_started(
    cascade: &FusedCascade,
    options: &RunOptions,
    task_run_id: &str,
    placement: &TaskPlacement,
) -> Result<()> {
    let metadata =
        crate::task_run_metadata::task_run_metadata_for_placement(&cascade.task, placement);
    emit_task_started(
        options.output_observer.as_ref(),
        TaskStartedEvent {
            task_run_id: task_run_id.to_string(),
            task_label: cascade.task.label.clone(),
            placement_mode: placement.placement_mode,
            remote_node_id: placement.remote_node_id.clone(),
            origin: Some(metadata.origin),
            runtime: metadata.runtime,
            runtime_source: metadata.runtime_source,
            command: metadata.command,
        },
    )
}

pub(super) fn emit_finished(
    task: &ResolvedTask,
    options: &RunOptions,
    result: &TaskRunResult,
) -> Result<()> {
    emit_task_finished(
        options.output_observer.as_ref(),
        TaskFinishedEvent {
            task_run_id: result.task_run_id.clone(),
            task_label: task.label.clone(),
            attempts: result.attempts,
            success: result.success,
            exit_code: result.exit_code,
            placement_mode: result.placement_mode,
            remote_node_id: result.remote_node_id.clone(),
        },
    )
}

pub(super) fn emit_failure(
    task: &ResolvedTask,
    options: &RunOptions,
    task_run_id: &str,
    placement: &TaskPlacement,
) -> Result<()> {
    emit_task_finished(
        options.output_observer.as_ref(),
        TaskFinishedEvent {
            task_run_id: task_run_id.to_string(),
            task_label: task.label.clone(),
            attempts: 1,
            success: false,
            exit_code: None,
            placement_mode: placement.placement_mode,
            remote_node_id: placement.remote_node_id.clone(),
        },
    )
}
