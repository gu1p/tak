use crate::support;

use base64::Engine;
use std::path::Path;
use std::process::Command as StdCommand;
use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn task_logs_prints_persisted_stdout_and_stderr_chunks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let key = register_task_with_logs(&store, temp.path(), "task-run-logs");

    store
        .set_result_payload(&key, r#"{"success":true}"#)
        .expect("complete task");
    store
        .append_event(&key, 4, r#"{"kind":"TASK_COMPLETED","timestamp_ms":4}"#)
        .expect("terminal event");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "task-run-logs",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs");

    assert!(output.status.success(), "takd task logs should succeed");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello stdout\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "hello stderr\n");
}

#[test]
fn task_logs_reports_missing_task_run_id() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "missing-run",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs");

    assert!(!output.status.success(), "missing task should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing-run"), "missing task id:\n{stderr}");
    assert!(
        stderr.contains("not found"),
        "missing actionable error:\n{stderr}"
    );
}

fn register_task_with_logs(store: &SubmitAttemptStore, temp: &Path, run_id: &str) -> String {
    let root = temp.join("exec");
    store
        .register_submit_with_task_label(run_id, Some(1), "//apps/web:test", "node-a", &root)
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
