use std::sync::Mutex;

use anyhow::Result;
use tak_core::model::TaskLabel;
use tak_exec::{
    TaskOutputChunk, TaskOutputObserver, TaskStatusEvent, TaskStatusEventKind, TaskStatusPhase,
    TaskStructuredStatusEvent,
};

#[derive(Default)]
struct LegacyObserver {
    events: Mutex<Vec<TaskStatusEvent>>,
}

impl TaskOutputObserver for LegacyObserver {
    fn observe_output(&self, _chunk: TaskOutputChunk) -> Result<()> {
        Ok(())
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        self.events.lock().expect("events").push(event);
        Ok(())
    }
}

#[test]
fn structured_status_delegates_to_legacy_callback_exactly_once() {
    let observer = LegacyObserver::default();
    observer
        .observe_structured_status(planned_event())
        .expect("observe planned event");

    let events = observer.events.lock().expect("events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].message, "planned");
    assert_eq!(events[0].phase, TaskStatusPhase::Scheduling);
}

fn planned_event() -> TaskStructuredStatusEvent {
    let label = TaskLabel {
        package: "//".into(),
        name: "lint".into(),
    };
    TaskStructuredStatusEvent {
        task_label: label.clone(),
        operation_name: "lint".into(),
        attempt: 0,
        phase: TaskStatusPhase::Scheduling,
        kind: TaskStatusEventKind::TaskPlanned,
        message: "planned".into(),
        timestamp_ms: 1,
        request_id: None,
        trace_id: None,
        local_daemon_path: None,
        transport: None,
        remote_node_id: None,
        queue_id: None,
        queue_position: None,
        eligible_worker_count: None,
        rejection_reason: None,
        original_error: None,
        retryable: None,
        bytes_total: None,
        bytes_sent: None,
        execution_unit_members: vec![label],
    }
}
