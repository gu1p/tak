use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::{RunOptions, TaskFinishedEvent, TaskRunResult, emit_task_finished};

pub(super) fn emit_finished(
    options: &RunOptions,
    task: &ResolvedTask,
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
