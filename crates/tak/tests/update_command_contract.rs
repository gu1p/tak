use crate::support;

use std::process::Command as StdCommand;
use takd::SubmitAttemptStore;

#[test]
fn tak_update_help_lists_flags() {
    let output = StdCommand::new(support::tak_bin())
        .args(["update", "--help"])
        .output()
        .expect("run tak update --help");
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--check"), "stdout: {stdout}");
    assert!(stdout.contains("--force"), "stdout: {stdout}");
    assert!(stdout.contains("--state-root"), "stdout: {stdout}");
    assert!(
        stdout.contains("active legacy attempts must finish"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("signed GitHub releases"),
        "stdout: {stdout}"
    );
}

#[test]
fn tak_update_refuses_to_replace_takd_while_legacy_work_is_active() {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let state_root = temp.path().join("state");
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite"))
        .expect("legacy submit store");
    store
        .register_submit_with_execution_root_base(
            "legacy-run",
            Some(1),
            "//:check",
            None,
            "worker-a",
            &temp.path().join("exec"),
        )
        .expect("register active legacy attempt");

    let output = StdCommand::new(support::tak_bin())
        .args(["update", "--state-root"])
        .arg(&state_root)
        .output()
        .expect("run tak update");

    assert!(!output.status.success(), "status: {:?}", output.status);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("active legacy attempts must finish"),
        "stderr: {stderr}"
    );
}
