use prost::Message;
use std::thread;
use std::time::Duration;
use tak_proto::{
    CmdStep, GetTaskResultResponse, NodeInfo, PollTaskEventsResponse, Step, SubmitTaskRequest,
    SubmitTaskResponse, SubmittedNeed, step,
};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_routes_serve_binary_protobuf_contracts() {
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
        },
        "secret".into(),
    );
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
        runtime: None,
        task_label: "//apps/web:test".to_string(),
        needs: vec![SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 1.0,
        }],
        outputs: Vec::new(),
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

fn empty_workspace_zip() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let writer = zip::ZipWriter::new(cursor);
    writer
        .finish()
        .expect("finish empty workspace zip")
        .into_inner()
}
