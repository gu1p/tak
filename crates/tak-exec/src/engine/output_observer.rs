use anyhow::Result;
use tak_core::model::TaskLabel;

use super::{
    OutputStream, TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStartedEvent,
    TaskStatusEvent, TaskStatusPhase,
};

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
    emit_task_status(
        output_observer,
        TaskStatusEvent {
            task_label: task_label.clone(),
            attempt,
            phase,
            remote_node_id: remote_node_id.map(str::to_string),
            message: message.into(),
        },
    )
}
