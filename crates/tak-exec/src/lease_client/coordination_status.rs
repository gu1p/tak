use std::time::Duration;

use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::{
    RunCancellation, RunOptions, TaskStatusDetails, TaskStatusEventKind, TaskStatusPhase,
    cancelled_error, emit_task_status_message_with_details,
};

#[derive(Default)]
pub(super) struct CoordinationQueueTracker {
    position: Option<usize>,
}

impl CoordinationQueueTracker {
    pub(super) fn pending(&mut self, position: usize) -> Option<TaskStatusEventKind> {
        let kind = match self.position {
            None => TaskStatusEventKind::QueueAdmission,
            Some(previous) if previous != position => TaskStatusEventKind::QueuePositionChanged,
            Some(_) => return None,
        };
        self.position = Some(position);
        Some(kind)
    }

    pub(super) fn granted(&mut self) -> Option<TaskStatusEventKind> {
        self.position.take().map(|_| TaskStatusEventKind::Dispatch)
    }
}

pub(super) async fn wait_for_retry_or_cancellation(
    duration: Duration,
    cancellation: &RunCancellation,
) -> Result<()> {
    tokio::select! {
        _ = tokio::time::sleep(duration) => Ok(()),
        _ = cancellation.cancelled() => Err(cancelled_error()),
    }
}

pub(super) fn emit_coordination_cancellation(
    task: &ResolvedTask,
    attempt: u32,
    options: &RunOptions,
    queue: &CoordinationQueueTracker,
) -> Result<()> {
    if queue.position.is_some() {
        emit_coordination_status(
            task,
            attempt,
            options,
            TaskStatusEventKind::Cancellation,
            queue.position,
            "coordination wait cancelled",
        )?;
    }
    Ok(())
}

pub(super) fn emit_coordination_status(
    task: &ResolvedTask,
    attempt: u32,
    options: &RunOptions,
    kind: TaskStatusEventKind,
    position: Option<usize>,
    message: impl Into<String>,
) -> Result<()> {
    emit_task_status_message_with_details(
        options.output_observer.as_ref(),
        &task.label,
        attempt,
        TaskStatusPhase::Scheduling,
        None,
        message,
        TaskStatusDetails {
            kind: Some(kind),
            queue_id: Some("coordination".into()),
            queue_position: position,
            ..TaskStatusDetails::default()
        },
    )
}
