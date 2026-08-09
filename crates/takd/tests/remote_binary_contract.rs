use crate::support;

use prost::Message;
use tak_proto::{GetTaskResultResponse, NodeInfo, NodePingResponse, SubmitTaskResponse};
use takd::SubmitAttemptStore;

use support::remote_output::test_context_with_runtime;

#[path = "remote_binary_contract/support.rs"]
mod contract_support;

#[test]
fn remote_routes_serve_binary_protobuf_contracts() {
    let _env_lock = support::env::env_lock();
    let mut env = support::env::EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let runtime_config = support::runtime_config::builder()
        .with_skip_exec_root_probe(true)
        .build();
    let context = test_context_with_runtime(runtime_config);
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let node = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/node/info",
        &[],
        None,
    )
    .expect("node info response");
    assert_eq!(node.content_type, "application/x-protobuf");
    let node_info = NodeInfo::decode(node.body.as_slice()).expect("decode node info");
    assert_eq!(node_info.node_id, "builder-a");
    let ping = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/node/ping",
        &[],
        None,
    )
    .expect("node ping response");
    assert_eq!(ping.content_type, "application/x-protobuf");
    let ping = NodePingResponse::decode(ping.body.as_slice()).expect("decode node ping");
    assert_eq!(ping.node_id, "builder-a");
    assert_eq!(ping.protocol_version, "v1");
    assert_eq!(ping.health, "healthy");
    let submit = contract_support::submit_request();
    let submit = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        &[],
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    let submit_ack = SubmitTaskResponse::decode(submit.body.as_slice()).expect("decode submit");
    assert!(submit_ack.accepted);
    contract_support::wait_for_terminal_events(&context, &store);

    let result = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/tasks/task-run-1/result",
        &[],
        None,
    )
    .expect("result response");
    let _ = GetTaskResultResponse::decode(result.body.as_slice()).expect("decode result");
}
