use anyhow::Result;
use tak_core::model::TaskLabel;

use super::{
    OutputStream, TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStartedEvent,
    TaskStatusEvent, TaskStatusEventKind, TaskStatusPhase, TaskStructuredStatusEvent,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct TaskStatusDetails {
    pub(crate) kind: Option<TaskStatusEventKind>,
    pub(crate) request_id: Option<String>,
    pub(crate) trace_id: Option<String>,
    pub(crate) local_daemon_path: Option<String>,
    pub(crate) transport: Option<String>,
    pub(crate) queue_id: Option<String>,
    pub(crate) queue_position: Option<usize>,
    pub(crate) eligible_worker_count: Option<usize>,
    pub(crate) rejection_reason: Option<String>,
    pub(crate) original_error: Option<String>,
    pub(crate) retryable: Option<bool>,
    pub(crate) bytes_total: Option<u64>,
    pub(crate) bytes_sent: Option<u64>,
}

pub(crate) fn emit_task_output(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_run_id: &str,
    task_label: &TaskLabel,
    attempt: u32,
    stream: OutputStream,
    bytes: &[u8],
) -> Result<()> {
    if bytes.is_empty() {
        return Ok(());
    }

    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_output(TaskOutputChunk {
        task_run_id: task_run_id.to_string(),
        task_label: task_label.clone(),
        attempt,
        stream,
        bytes: bytes.to_vec(),
    })
}

pub(crate) fn emit_task_started(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    event: TaskStartedEvent,
) -> Result<()> {
    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_task_started(event)
}

pub(crate) fn emit_task_finished(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    event: TaskFinishedEvent,
) -> Result<()> {
    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_task_finished(event)
}

pub(crate) fn emit_task_status(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    event: TaskStatusEvent,
) -> Result<()> {
    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_status(event)
}

pub(crate) fn emit_task_status_message(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    phase: TaskStatusPhase,
    remote_node_id: Option<&str>,
    message: impl Into<String>,
) -> Result<()> {
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        phase,
        remote_node_id,
        message,
        TaskStatusDetails::default(),
    )
}

pub(crate) fn emit_task_status_message_with_details(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    phase: TaskStatusPhase,
    remote_node_id: Option<&str>,
    message: impl Into<String>,
    details: TaskStatusDetails,
) -> Result<()> {
    let message = message.into();
    emit_task_status(
        output_observer,
        TaskStatusEvent {
            task_label: task_label.clone(),
            attempt,
            phase,
            remote_node_id: remote_node_id.map(str::to_string),
            message: message.clone(),
        },
    )?;
    emit_structured_task_status(
        output_observer,
        task_label,
        attempt,
        phase,
        remote_node_id,
        message,
        details,
    )
}

fn emit_structured_task_status(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    phase: TaskStatusPhase,
    remote_node_id: Option<&str>,
    message: String,
    details: TaskStatusDetails,
) -> Result<()> {
    let Some(kind) = details.kind else {
        return Ok(());
    };
    let Some(observer) = output_observer else {
        return Ok(());
    };
    observer.observe_structured_status(TaskStructuredStatusEvent {
        task_label: task_label.clone(),
        operation_name: task_label.name.clone(),
        attempt,
        phase,
        kind,
        message,
        timestamp_ms: unix_epoch_ms(),
        request_id: details.request_id,
        trace_id: details.trace_id,
        local_daemon_path: details.local_daemon_path,
        transport: details.transport,
        remote_node_id: remote_node_id.map(str::to_string),
        queue_id: details.queue_id,
        queue_position: details.queue_position,
        eligible_worker_count: details.eligible_worker_count,
        rejection_reason: details.rejection_reason,
        original_error: details.original_error,
        retryable: details.retryable,
        bytes_total: details.bytes_total,
        bytes_sent: details.bytes_sent,
    })
}

fn unix_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
