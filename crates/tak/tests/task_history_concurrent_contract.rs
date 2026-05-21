use crate::support;

use std::fs;
use std::process::{Child, Command as StdCommand, Stdio};

#[test]
fn task_history_keeps_two_concurrent_local_runs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(
        workspace.join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("concurrent", steps=[cmd("sh", "-c", "printf 'begin\n'; sleep 0.2; printf 'end\n'")]),
])
SPEC
"#,
    )
    .expect("write tasks");

    let first = spawn_run(&workspace, &state_root);
    let second = spawn_run(&workspace, &state_root);
    assert_success(first.wait_with_output().expect("wait first"), "first");
    assert_success(second.wait_with_output().expect("wait second"), "second");

    let output = StdCommand::new(support::tak_bin())
        .args(["task", "list", "--limit", "8"])
        .current_dir(&workspace)
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
    assert!(
        stdout.matches("task_label=//:concurrent").count() >= 2,
        "stdout:\n{stdout}"
    );
}

fn spawn_run(workspace: &std::path::Path, state_root: &std::path::Path) -> Child {
    StdCommand::new(support::tak_bin())
        .args(["run", "//:concurrent"])
        .current_dir(workspace)
        .env("XDG_STATE_HOME", state_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tak run")
}

fn assert_success(output: std::process::Output, name: &str) {
    assert!(
        output.status.success(),
        "{name} run should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
