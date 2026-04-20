use prost::Message;
use tak_proto::{CancelTaskResponse, NodeInfo};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_cancel_route_serves_protobuf_response() {
    let context = RemoteNodeContext::new(
        NodeInfo {
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
        RemoteRuntimeConfig::for_tests(),
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let response = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/task-run-9/cancel",
        None,
    )
    .expect("cancel response");
    assert_eq!(response.status_code, 202);
    let cancel =
        CancelTaskResponse::decode(response.body.as_slice()).expect("decode cancel response");
    assert!(cancel.cancelled);
    assert_eq!(cancel.task_run_id, "task-run-9");
}
