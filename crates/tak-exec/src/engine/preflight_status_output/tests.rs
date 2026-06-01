use std::sync::{Arc, Mutex};

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::{
    TaskOutputObserver, TaskStatusEvent, TaskStatusEventKind, TaskStructuredStatusEvent,
};

use super::emit_remote_probe;

#[derive(Default)]
struct StatusCollector {
    human: Mutex<Vec<TaskStatusEvent>>,
    structured: Mutex<Vec<TaskStructuredStatusEvent>>,
}

impl StatusCollector {
    fn human_messages(&self) -> Vec<TaskStatusEvent> {
        self.human.lock().expect("human status lock").clone()
    }

    fn structured_events(&self) -> Vec<TaskStructuredStatusEvent> {
        self.structured
            .lock()
            .expect("structured status lock")
            .clone()
    }
}

impl TaskOutputObserver for StatusCollector {
    fn observe_output(&self, _chunk: crate::engine::TaskOutputChunk) -> Result<()> {
        Ok(())
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        self.human.lock().expect("human status lock").push(event);
        Ok(())
    }

    fn observe_structured_status(&self, event: TaskStructuredStatusEvent) -> Result<()> {
        self.structured
            .lock()
            .expect("structured status lock")
            .push(event);
        Ok(())
    }
}

#[test]
fn daemon_tor_probe_status_names_local_daemon_not_placeholder_node() {
    let observer = Arc::new(StatusCollector::default());
    let collector = observer.clone();
    let observer: Arc<dyn TaskOutputObserver> = observer;
    let label = label();

    emit_remote_probe(Some(&observer), &label, 1, "__takd_daemon_tor__").expect("emit probe");

    let human = collector.human_messages();
    assert_eq!(human.len(), 1);
    assert_eq!(human[0].remote_node_id, None);
    assert!(human[0].message.contains("local takd daemon"));
    assert!(!human[0].message.contains("__takd_daemon_tor__"));
    assert!(!human[0].message.contains("remote node __takd_daemon_tor__"));

    let structured = collector.structured_events();
    assert_eq!(structured.len(), 1);
    assert_eq!(
        structured[0].kind,
        TaskStatusEventKind::LocalDaemonConnection
    );
    assert_eq!(structured[0].remote_node_id, None);
    assert_eq!(structured[0].transport.as_deref(), Some("tor"));
}

fn label() -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: "check".into(),
    }
}
