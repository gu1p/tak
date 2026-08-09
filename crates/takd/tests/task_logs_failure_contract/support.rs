use crate::support;

use std::process::{Command as StdCommand, Output};
use takd::daemon::remote::SubmitAttemptStore;

pub(super) fn run_task_logs_with_terminal_event(run_id: &str, terminal_event: &str) -> Output {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    let root = temp.path().join("exec");
    store
        .register_submit_with_execution_root_base(run_id, Some(1), "//:test", None, "node-a", &root)
        .expect("register task");
    let key = store
        .latest_submit_idempotency_key_for_task_run(run_id)
        .expect("key")
        .expect("key exists");
    store
        .append_event(&key, 1, r#"{"kind":"TASK_STARTED","timestamp_ms":1}"#)
        .expect("start event");
    store
        .append_event(&key, 2, terminal_event)
        .expect("terminal event");

    StdCommand::new(support::takd_bin())
        .args([
            "task",
            "logs",
            run_id,
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd task logs")
}
