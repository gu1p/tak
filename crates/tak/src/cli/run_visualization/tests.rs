#[path = "tests/framing.rs"]
mod framing;
#[path = "tests/model.rs"]
mod model;
#[path = "tests/render.rs"]
mod render;
#[path = "tests/terminal.rs"]
mod terminal;

use tak_core::model::TaskLabel;
use tak_exec::{TaskStatusEventKind, TaskStatusPhase, TaskStructuredStatusEvent};

use super::model::{RunState, TaskRow};

fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}

fn row<'a>(state: &'a RunState, label: &TaskLabel) -> &'a TaskRow {
    state.rows.get(label).expect("row")
}

fn backdate(state: &mut RunState, label: &TaskLabel, duration: std::time::Duration) {
    state.rows.get_mut(label).expect("row").started_at -= duration;
}

fn status(
    name: &str,
    kind: TaskStatusEventKind,
    queue_id: Option<&str>,
    queue_position: Option<usize>,
    node: Option<&str>,
) -> TaskStructuredStatusEvent {
    TaskStructuredStatusEvent {
        task_label: label(name),
        operation_name: name.into(),
        attempt: 1,
        phase: if kind == TaskStatusEventKind::TaskPlanned {
            TaskStatusPhase::Scheduling
        } else {
            TaskStatusPhase::RemoteWait
        },
        kind,
        message: "status message".into(),
        timestamp_ms: 1,
        request_id: None,
        trace_id: None,
        local_daemon_path: None,
        transport: None,
        remote_node_id: node.map(str::to_string),
        queue_id: queue_id.map(str::to_string),
        queue_position,
        eligible_worker_count: None,
        rejection_reason: None,
        original_error: None,
        retryable: None,
        bytes_total: None,
        bytes_sent: None,
        execution_unit_members: vec![label(name)],
    }
}
