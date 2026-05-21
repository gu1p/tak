//! Contract test for `tak run` staying client-side.

use crate::support;

use std::collections::BTreeMap;
use std::fs;
use std::process::Command as StdCommand;

use support::run_tak_output;

#[path = "run_local_contract/container_runtime.rs"]
mod container_runtime;

#[test]
fn run_command_executes_locally_without_takd() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker = temp.path().join("local-run.log");
    fs::write(
        temp.path().join("TASKS.py"),
        format!(
            r#"
SPEC = module_spec(tasks=[
  task("local_only", steps=[cmd("sh", "-c", "echo client-side-run > {marker}")]),
])
SPEC
"#,
            marker = marker.display()
        ),
    )
    .expect("write tasks");

    let output = StdCommand::new(support::tak_bin())
        .args(["run", "//:local_only"])
        .current_dir(temp.path())
        .output()
        .expect("run command");

    assert!(
        output.status.success(),
        "run should succeed locally\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(marker).expect("read marker").trim(),
        "client-side-run"
    );
}

#[test]
fn run_command_reports_task_start_before_silent_local_task() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("silent", steps=[cmd("sh", "-c", ":")]),
])
SPEC
"#,
    )
    .expect("write tasks");

    let env = BTreeMap::new();
    let output = run_tak_output(temp.path(), &["run", "//:silent"], &env).expect("run");

    assert!(
        output.status.success(),
        "run should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("//:silent: started"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
