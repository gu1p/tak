use std::fs;

use prost::Message;
use tak_proto::ErrorResponse;
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

mod support;

use support::env::{EnvGuard, env_lock};

#[test]
fn remote_output_route_decodes_percent_encoded_workspace_paths() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("takd-remote-exec");
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        exec_root_base.display().to_string(),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite")).expect("store");
    let context = RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
    );

    let registration = store
        .register_submit("run-1", Some(1), "builder-a")
        .expect("register submit");
    let idempotency_key = match registration {
        takd::SubmitRegistration::Created { idempotency_key }
        | takd::SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    };

    let artifact_root = temp
        .path()
        .join("takd-remote-artifacts")
        .join(idempotency_key.replace(':', "_"));
    let _ = fs::remove_dir_all(&artifact_root);
    let nested = artifact_root.join("nested");
    fs::create_dir_all(&nested).expect("artifact dirs");
    fs::write(nested.join("output.txt"), b"hello from artifact").expect("artifact file");

    let response = handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/run-1/outputs?path=nested%2Foutput.txt",
        None,
    )
    .expect("route response");

    if response.status_code != 200 {
        let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
        panic!(
            "expected output fetch to succeed, got {} with {:?}",
            response.status_code, error
        );
    }
    assert_eq!(response.content_type, "application/octet-stream");
    assert_eq!(response.body, b"hello from artifact");

    let _ = fs::remove_dir_all(artifact_root);
}
