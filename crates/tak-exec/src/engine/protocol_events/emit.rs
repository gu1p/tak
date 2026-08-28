//! Emits event-stream output: the per-attempt wait status plus each parsed
//! batch of status updates and log chunks.

use std::sync::Arc;

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::output_observer::{
    TaskStatusDetails, emit_task_status_message, emit_task_status_message_with_details,
};
use crate::engine::remote_models::RemoteStatusUpdate;
use crate::engine::{
    RemoteLogChunk, StrictRemoteTarget, TaskOutputObserver, TaskStatusPhase, emit_task_output,
};

pub(super) fn emit_remote_wait(
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
    target: &StrictRemoteTarget,
    task_label: &TaskLabel,
    attempt: u32,
    message: String,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteWait,
        Some(target.node_id.as_str()),
        message,
    )
}

pub(super) fn emit_event_batch(
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
    target: &StrictRemoteTarget,
    task_label: &TaskLabel,
    task_run_id: &str,
    attempt: u32,
    status_updates: &[RemoteStatusUpdate],
    remote_logs: &[RemoteLogChunk],
) -> Result<()> {
    for update in status_updates {
        emit_task_status_message_with_details(
            output_observer,
            task_label,
            attempt,
            TaskStatusPhase::RemoteWait,
            Some(target.node_id.as_str()),
            update.message.clone(),
            TaskStatusDetails {
                kind: Some(update.kind),
                queue_id: matches!(
                    update.kind,
                    crate::engine::TaskStatusEventKind::QueueAdmission
                        | crate::engine::TaskStatusEventKind::QueuePositionChanged
                )
                .then(|| "worker".to_string()),
                queue_position: update.queue_position,
                transport: Some(target.transport_kind.as_result_value().to_string()),
                ..TaskStatusDetails::default()
            },
        )?;
    }
    for chunk in remote_logs {
        emit_task_output(
            output_observer,
            task_run_id,
            task_label,
            attempt,
            chunk.stream,
            &chunk.bytes,
        )?;
    }
    Ok(())
}
