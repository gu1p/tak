use crate::support;

use std::process::Command as StdCommand;

#[cfg(unix)]
#[test]
fn task_list_reads_existing_read_only_history_without_schema_write() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::write_active_container_run(&state_root);
    support::task_history::make_db_read_only(&state_root);

    let output = StdCommand::new(support::tak_bin())
        .args(["task", "list", "--limit", "8"])
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak task list");

    assert!(
        output.status.success(),
        "tak task list should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local Tasks"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("task_label=//apps/web:build"),
        "stdout:\n{stdout}"
    );
}

#[test]
fn task_list_reports_unavailable_history_without_failing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::create_unopenable_db_path(&state_root);

    let output = StdCommand::new(support::tak_bin())
        .args(["task", "list", "--limit", "8"])
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak task list");

    assert!(
        output.status.success(),
        "tak task list should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("history=unavailable"), "stdout:\n{stdout}");
}
