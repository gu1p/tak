use tak_core::v2::{ContainerSource, TaskRuntime, WorkspaceEntry};
use tak_proto::worker_v2::{
    WorkerWorkspaceOverlay, WorkerWorkspaceReuse, decode_dispatch_request, encode_dispatch_request,
};

use crate::worker_v2_attempt_support::{payload, request};

#[test]
fn worker_dispatch_is_strictly_v2_and_binds_the_canonical_payload_digest() {
    let payload = payload();
    let request = request(payload);
    let encoded = encode_dispatch_request(&request).unwrap();
    assert_eq!(decode_dispatch_request(&encoded).unwrap(), request);

    let mut stale_version = serde_json::to_value(&request).unwrap();
    stale_version["protocol_version"] = 1.into();
    assert!(decode_dispatch_request(&serde_json::to_vec(&stale_version).unwrap()).is_err());
    let mut changed = request.clone();
    changed.payload.tasks[0].task_id = "//:changed".into();
    assert!(decode_dispatch_request(&serde_json::to_vec(&changed).unwrap()).is_err());
}

#[test]
fn worker_overlays_are_canonical_content_verified_and_session_safe() {
    let entry = |path| {
        WorkspaceEntry::file(
            path,
            false,
            0,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        )
        .unwrap()
    };
    let mut payload = payload();
    payload.workspace.overlays = vec![
        WorkerWorkspaceOverlay {
            entry: entry("b"),
            content_base64: Some(String::new()),
        },
        WorkerWorkspaceOverlay {
            entry: entry("a"),
            content_base64: Some(String::new()),
        },
    ];
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());
    payload
        .workspace
        .overlays
        .sort_by(|left, right| left.entry.path.cmp(&right.entry.path));
    payload.workspace.overlays[0].content_base64 = None;
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());
    payload.workspace.overlays[0].content_base64 = Some("eA==".into());
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());
    payload.workspace.overlays[0].content_base64 = Some(String::new());
    payload.workspace_reuse = WorkerWorkspaceReuse::Shared {
        session_id: String::new(),
        affinity_group: "shared-group".into(),
    };
    assert!(encode_dispatch_request(&request(payload)).is_err());
}

#[test]
fn worker_dispatch_preserves_task_timeout_and_runtime() {
    let mut payload = payload();
    payload.tasks[0].timeout_s = Some(17);
    payload.tasks[0].runtime = Some(TaskRuntime::container(ContainerSource::Dockerfile {
        dockerfile: "docker/Dockerfile".into(),
        build_context: "docker".into(),
    }));
    let request = request(payload);

    let decoded = decode_dispatch_request(&encode_dispatch_request(&request).unwrap()).unwrap();
    assert_eq!(decoded, request);
}
