use crate::support;

use std::process::Command as StdCommand;
use tak_proto::worker_v2::WorkerAttemptState;
use takd::daemon::remote::SubmitAttemptStore;

#[path = "task_logs_contract/support.rs"]
mod contract_support;

#[test]
fn task_logs_prints_persisted_stdout_and_stderr_chunks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let key = contract_support::register_task_with_logs(&store, temp.path(), "task-run-logs");

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

#[test]
fn task_logs_read_handle_does_not_mark_live_v2_work_missing() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let request = support::v2_worker::dispatch(1, 1, "live-fence");
    store.register_worker_v2_attempt(&request).unwrap();
    let key = contract_support::register_task_with_logs(&store, temp.path(), "legacy-logs");
    store
        .set_result_payload(&key, r#"{"success":true}"#)
        .unwrap();

    let output = StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            "legacy-logs",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs");

    assert!(output.status.success());
    assert_eq!(
        store
            .observe_worker_v2_attempt(&request.identity, 0)
            .unwrap()
            .state,
        WorkerAttemptState::Running
    );
}
