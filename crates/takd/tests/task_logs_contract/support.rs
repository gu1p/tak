use std::path::Path;

use base64::Engine;
use takd::daemon::remote::SubmitAttemptStore;

pub(super) fn register_task_with_logs(
    store: &SubmitAttemptStore,
    temp: &Path,
    run_id: &str,
) -> String {
    let root = temp.join("exec");
    store
        .register_submit_with_execution_root_base(
            run_id,
            Some(1),
            "//apps/web:test",
            None,
            "node-a",
            &root,
        )
        .expect("register task");
    let key = store
        .latest_submit_idempotency_key_for_task_run(run_id)
        .expect("key")
        .expect("key exists");
    store
        .append_event(&key, 1, r#"{"kind":"TASK_STARTED","timestamp_ms":1}"#)
        .expect("start event");
    store
        .append_event(
            &key,
            2,
            &chunk_payload("TASK_STDOUT_CHUNK", b"hello stdout\n"),
        )
        .expect("stdout event");
    store
        .append_event(
            &key,
            3,
            &chunk_payload("TASK_STDERR_CHUNK", b"hello stderr\n"),
        )
        .expect("stderr event");
    key
}

fn chunk_payload(kind: &str, bytes: &[u8]) -> String {
    serde_json::json!({
        "kind": kind,
        "timestamp_ms": 2,
        "chunk_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
    })
    .to_string()
}
