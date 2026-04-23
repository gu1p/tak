use prost::Message;
use tak_proto::SubmitTaskResponse;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::remote_binary::{
    streaming_context, streaming_submit_request_with_command, wait_for_streaming_events_for_task,
};

#[test]
fn remote_routes_round_trip_non_utf8_chunk_bytes_without_persisting_lossy_chunk_text() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let context = streaming_context();
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit = streaming_submit_request_with_command(
        "task-run-stream-non-utf8",
        "printf '\\377stdout\\n'; printf '\\200stderr\\n' >&2",
    );
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

    let events = wait_for_streaming_events_for_task(&context, &store, "task-run-stream-non-utf8");
    let stdout_event = events
        .events
        .iter()
        .find(|event| event.kind == "TASK_STDOUT_CHUNK")
        .expect("stdout event");
    assert_eq!(stdout_event.chunk_bytes, b"\xffstdout\n");
    assert_eq!(stdout_event.chunk, None);

    let stderr_event = events
        .events
        .iter()
        .find(|event| event.kind == "TASK_STDERR_CHUNK")
        .expect("stderr event");
    assert_eq!(stderr_event.chunk_bytes, b"\x80stderr\n");
    assert_eq!(stderr_event.chunk, None);

    let stored_events = store
        .events(&submit_ack.idempotency_key)
        .expect("stored events");
    let chunk_payloads = stored_events
        .iter()
        .map(|event| {
            serde_json::from_str::<serde_json::Value>(&event.payload_json).expect("event payload")
        })
        .filter(|payload| {
            matches!(
                payload.get("kind").and_then(serde_json::Value::as_str),
                Some("TASK_STDOUT_CHUNK" | "TASK_STDERR_CHUNK")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(chunk_payloads.len(), 2);
    for payload in chunk_payloads {
        assert!(payload.get("chunk_base64").is_some(), "{payload}");
        assert!(payload.get("chunk").is_none(), "{payload}");
    }
}
