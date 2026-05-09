use crate::support;

use std::process::Command as StdCommand;

#[test]
fn local_status_lists_active_containerized_tasks_from_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::write_active_container_run(&state_root);

    let output = StdCommand::new(support::tak_bin())
        .args(["local", "status"])
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak local status");

    assert!(
        output.status.success(),
        "tak local status should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local"), "missing local section:\n{stdout}");
    assert!(
        stdout.contains("Containers"),
        "missing containers section:\n{stdout}"
    );
    assert!(
        stdout.contains("//apps/web:build"),
        "missing task label:\n{stdout}"
    );
    assert!(
        stdout.contains("command=make build"),
        "missing command metadata:\n{stdout}"
    );
    assert!(
        !stdout.contains("\x1b["),
        "captured local status output should stay plain:\n{stdout}"
    );
}

#[test]
fn local_status_excludes_remote_task_history() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::write_active_remote_container_run(&state_root);

    let output = StdCommand::new(support::tak_bin())
        .args(["local", "status"])
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak local status");

    assert!(
        output.status.success(),
        "tak local status should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("active_tasks=0 containers=0"),
        "remote history should not count as local activity:\n{stdout}"
    );
    assert!(
        !stdout.contains("//apps/remote:build"),
        "remote task should not appear in local status:\n{stdout}"
    );
    assert!(
        !stdout.contains("remote_node=builder-a"),
        "remote node metadata should not appear in local status:\n{stdout}"
    );
}
