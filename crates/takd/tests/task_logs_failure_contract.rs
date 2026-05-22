use crate::support;

use std::process::Command as StdCommand;
use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn task_logs_prints_terminal_failure_message_to_stderr() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");
    store
        .register_submit_with_task_label("task-run-failed", Some(1), "//:test", "node-a", &root)
        .expect("register task");
    let key = store
        .latest_submit_idempotency_key_for_task_run("task-run-failed")
        .expect("key")
        .expect("key exists");
    store
        .append_event(&key, 1, r#"{"kind":"TASK_STARTED","timestamp_ms":1}"#)
        .expect("start event");
    store
        .append_event(
            &key,
            2,
            r#"{"kind":"TASK_FAILED","timestamp_ms":2,"exit_code":137}"#,
        )
        .expect("terminal event");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "task-run-failed",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs");

    assert!(output.status.success(), "takd task logs should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "remote task failed with exit code 137\n"
    );
}

#[test]
fn task_logs_prints_terminal_cancelled_exit_code_to_stderr() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");
    store
        .register_submit_with_task_label("task-run-cancelled", Some(1), "//:test", "node-a", &root)
        .expect("register task");
    let key = store
        .latest_submit_idempotency_key_for_task_run("task-run-cancelled")
        .expect("key")
        .expect("key exists");
    store
        .append_event(&key, 1, r#"{"kind":"TASK_STARTED","timestamp_ms":1}"#)
        .expect("start event");
    store
        .append_event(
            &key,
            2,
            r#"{"kind":"TASK_CANCELLED","timestamp_ms":2,"success":false,"exit_code":137}"#,
        )
        .expect("terminal event");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "task-run-cancelled",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs");

    assert!(output.status.success(), "takd task logs should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "remote task cancelled with exit code 137\n"
    );
}
