use prost::Message;
use tak_proto::SubmitTaskResponse;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

mod support;

use support::remote_binary::{
    streaming_context, streaming_submit_request, wait_for_streaming_events,
};

#[test]
fn remote_routes_stream_stdout_and_stderr_events_before_terminal_result() {
    let context = streaming_context();
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit = streaming_submit_request();
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

    let events = wait_for_streaming_events(&context, &store);
    let kinds = events
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"TASK_STARTED"));
    assert!(kinds.contains(&"TASK_STDOUT_CHUNK"));
    assert!(kinds.contains(&"TASK_STDERR_CHUNK"));
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(*kind, "TASK_COMPLETED" | "TASK_FAILED"))
    );

    let stdout_event = events
        .events
        .iter()
        .find(|event| event.kind == "TASK_STDOUT_CHUNK")
        .expect("stdout event");
    assert_eq!(stdout_event.chunk_bytes, b"remote-stdout\n");

    let stderr_event = events
        .events
        .iter()
        .find(|event| event.kind == "TASK_STDERR_CHUNK")
        .expect("stderr event");
    assert_eq!(stderr_event.chunk_bytes, b"remote-stderr\n");
}
