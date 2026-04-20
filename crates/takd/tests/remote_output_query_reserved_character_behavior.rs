use std::fs;

use prost::Message;
use tak_proto::ErrorResponse;
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_output_route_preserves_literal_plus_and_percent_sequences_when_query_is_encoded() {
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root_base = temp.path().join("takd-remote-exec");
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
        RemoteRuntimeConfig::for_tests().with_explicit_remote_exec_root(exec_root_base.clone()),
    );

    let registration = store
        .register_submit("run-1", Some(1), "builder-a", &exec_root_base)
        .expect("register submit");
    let idempotency_key = match registration {
        takd::SubmitRegistration::Created { idempotency_key }
        | takd::SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    };

    let raw_path = "nested/a+b%2Fc.txt";
    let artifact_root = temp
        .path()
        .join("takd-remote-artifacts")
        .join(idempotency_key.replace(':', "_"));
    let nested = artifact_root.join("nested");
    fs::create_dir_all(&nested).expect("artifact dirs");
    fs::write(nested.join("a+b%2Fc.txt"), b"hello reserved path").expect("artifact file");

    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("path", raw_path)
        .finish();
    let request_path = format!("/v1/tasks/run-1/outputs?{query}");
    let response = handle_remote_v1_request(&context, &store, "GET", &request_path, None)
        .expect("route response");

    if response.status_code != 200 {
        let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
        panic!(
            "expected output fetch to succeed, got {} with {:?}",
            response.status_code, error
        );
    }
    assert_eq!(response.content_type, "application/octet-stream");
    assert_eq!(response.body, b"hello reserved path");
}
