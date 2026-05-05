use crate::support;

use base64::Engine;
use std::process::Command as StdCommand;
use std::{thread, time::Duration};
use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn task_logs_follow_streams_new_chunks_until_terminal_event() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");
    store
        .register_submit_with_task_label(
            "task-run-follow",
            Some(1),
            "//apps/web:test",
            "node-a",
            &root,
        )
        .expect("register task");
    let key = store
        .latest_submit_idempotency_key_for_task_run("task-run-follow")
        .expect("key")
        .expect("key exists");
    store
        .append_event(&key, 1, r#"{"kind":"TASK_STARTED","timestamp_ms":1}"#)
        .expect("start event");

    let writer_store = store.clone();
    let writer_key = key.clone();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        writer_store
            .append_event(
                &writer_key,
                2,
                &chunk_payload("TASK_STDOUT_CHUNK", b"follow stdout\n"),
            )
            .expect("stdout event");
        writer_store
            .set_result_payload(&writer_key, r#"{"success":true}"#)
            .expect("complete task");
        writer_store
            .append_event(
                &writer_key,
                3,
                r#"{"kind":"TASK_COMPLETED","timestamp_ms":3}"#,
            )
            .expect("terminal event");
    });

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "task-run-follow",
            "--state-root",
            &state_root.display().to_string(),
            "--follow",
            "--interval-ms",
            "10",
        ])
        .env("TAKD_TEST_TASK_LOGS_MAX_POLLS", "200")
        .output()
        .expect("run takd task logs --follow");
    writer.join().expect("writer exits");

    assert!(
        output.status.success(),
        "takd task logs --follow should succeed"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "follow stdout\n");
}

fn chunk_payload(kind: &str, bytes: &[u8]) -> String {
    serde_json::json!({
        "kind": kind,
        "timestamp_ms": 2,
        "chunk_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
    })
    .to_string()
}
