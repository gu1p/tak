use std::sync::{Arc, Mutex};

use anyhow::Result;
use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind, TaskLabel};

use crate::engine::workspace_upload_tor_stream_test_support::EnvVarGuard;
use crate::engine::{
    RemoteWorkspaceStage, StrictRemoteTarget, TaskOutputObserver, TaskStatusEvent,
    TaskStatusEventKind, TaskStructuredStatusEvent,
};

use super::start_upload_progress;

#[derive(Default)]
struct StatusCollector {
    human: Mutex<Vec<TaskStatusEvent>>,
    structured: Mutex<Vec<TaskStructuredStatusEvent>>,
}

impl StatusCollector {
    fn human_messages(&self) -> Vec<TaskStatusEvent> {
        self.human.lock().unwrap().clone()
    }

    fn structured_events(&self) -> Vec<TaskStructuredStatusEvent> {
        self.structured.lock().unwrap().clone()
    }
}

impl TaskOutputObserver for StatusCollector {
    fn observe_output(&self, _chunk: crate::engine::TaskOutputChunk) -> Result<()> {
        Ok(())
    }

    fn observe_status(&self, event: TaskStatusEvent) -> Result<()> {
        self.human.lock().unwrap().push(event);
        Ok(())
    }

    fn observe_structured_status(&self, event: TaskStructuredStatusEvent) -> Result<()> {
        self.structured.lock().unwrap().push(event);
        Ok(())
    }
}

#[test]
fn daemon_tor_upload_start_names_default_wormhole_transfer() {
    let _env_lock = crate::engine::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");
    let observer = Arc::new(StatusCollector::default());
    let collector = observer.clone();
    let observer: Arc<dyn TaskOutputObserver> = observer;
    let target = StrictRemoteTarget::daemon_tor_placement(&remote_spec());
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
    assert!(human[0].message.contains("through Magic Wormhole"));
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
