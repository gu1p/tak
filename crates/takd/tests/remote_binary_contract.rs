use crate::support;

use prost::Message;
use std::thread;
use std::time::Duration;
use tak_proto::{
    CmdStep, GetTaskResultResponse, NodeInfo, PollTaskEventsResponse, Step, SubmitTaskRequest,
    SubmitTaskResponse, SubmittedNeed, step,
};
use takd::{RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

use support::remote_output::{
    empty_workspace_zip, test_container_runtime, test_context_with_runtime,
};

#[test]
fn remote_routes_serve_binary_protobuf_contracts() {
    let _env_lock = support::env::env_lock();
    let mut env = support::env::EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let context =
        test_context_with_runtime(RemoteRuntimeConfig::for_tests().with_skip_exec_root_probe(true));
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let node = handle_remote_v1_request(&context, &store, "GET", "/v1/node/info", None)
        .expect("node info response");
    assert_eq!(node.content_type, "application/x-protobuf");
    let node_info = NodeInfo::decode(node.body.as_slice()).expect("decode node info");
    assert_eq!(node_info.node_id, "builder-a");
    let submit = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), "true".to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:test".to_string(),
        needs: vec![SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 1.0,
        }],
        outputs: Vec::new(),
        session: None,
    };
    let submit = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    let submit_ack = SubmitTaskResponse::decode(submit.body.as_slice()).expect("decode submit");
    assert!(submit_ack.accepted);
    for _ in 0..50 {
        let events =
            handle_remote_v1_request(&context, &store, "GET", "/v1/tasks/task-run-1/events", None)
                .expect("events response");
        let events = PollTaskEventsResponse::decode(events.body.as_slice()).expect("decode events");
        if events.done {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let result =
        handle_remote_v1_request(&context, &store, "GET", "/v1/tasks/task-run-1/result", None)
            .expect("result response");
    let _ = GetTaskResultResponse::decode(result.body.as_slice()).expect("decode result");
}
