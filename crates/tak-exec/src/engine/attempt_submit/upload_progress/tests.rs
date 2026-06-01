use std::sync::{Arc, Mutex};

use anyhow::Result;
use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind, TaskLabel};

use crate::engine::{
    RemoteWorkspaceStage, TaskOutputObserver, TaskStatusEvent, TaskStatusEventKind,
    TaskStructuredStatusEvent,
};

use super::start_upload_progress;

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
fn daemon_tor_upload_start_is_relay_scoped_until_worker_is_known() {
    let observer = Arc::new(StatusCollector::default());
    let collector = observer.clone();
    let observer: Arc<dyn TaskOutputObserver> = observer;
    let target = crate::engine::StrictRemoteTarget::daemon_tor_placement(&remote_spec());
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let workspace = RemoteWorkspaceStage {
        archive_path: temp_dir.path().join("workspace.zip"),
        archive_byte_len: 3_770_000,
        sha256: "abc123".into(),
        manifest_hash: "manifest".into(),
        temp_dir,
    };

    let label = TaskLabel {
        package: "//".into(),
        name: "check".into(),
    };

    start_upload_progress(Some(&observer), &label, 1, &target, &workspace)
        .expect("start upload progress");

    let human = collector.human_messages();
    assert_eq!(human.len(), 1);
    assert_eq!(human[0].remote_node_id, None);
    assert!(human[0].message.contains("through local takd Tor relay"));
    assert!(!human[0].message.contains("__takd_daemon_tor__"));
    assert!(!human[0].message.contains("to remote node"));

    let structured = collector.structured_events();
    assert_eq!(structured.len(), 1);
    assert_eq!(structured[0].kind, TaskStatusEventKind::UploadStart);
    assert_eq!(structured[0].remote_node_id, None);
    assert_eq!(structured[0].transport.as_deref(), Some("tor"));
    assert_eq!(structured[0].bytes_total, Some(3_770_000));
}

fn remote_spec() -> RemoteSpec {
    RemoteSpec {
        pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Tor,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    }
}
