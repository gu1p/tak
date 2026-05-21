use crate::support;

use std::collections::BTreeMap;
use std::fs;

use support::run_tak_output;

#[test]
fn run_continues_when_local_task_history_is_unavailable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::create_unopenable_db_path(&state_root);
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("history_unavailable", steps=[cmd("sh", "-c", "printf 'still ran\n'")]),
])
SPEC
"#,
    )
    .expect("write tasks");

    let env = history_env(&state_root);
    let output =
        run_tak_output(temp.path(), &["run", "//:history_unavailable"], &env).expect("run tak");

    assert!(
        output.status.success(),
        "tak run should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("still ran"),
        "stdout:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("local task history unavailable"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_status_reports_unavailable_history_without_failing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::create_unopenable_db_path(&state_root);

    let env = history_env(&state_root);
    let output = run_tak_output(temp.path(), &["local", "status"], &env).expect("run tak");

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local"), "stdout:\n{stdout}");
    assert!(stdout.contains("history=unavailable"), "stdout:\n{stdout}");
}

#[test]
fn docker_ps_reports_unavailable_local_history_without_failing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    support::task_history::create_unopenable_db_path(&state_root);

    let env = history_env(&state_root);
    let output = run_tak_output(temp.path(), &["--local", "docker", "ps"], &env).expect("run tak");

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tak Containers"), "stdout:\n{stdout}");
    assert!(stdout.contains("history=unavailable"), "stdout:\n{stdout}");
}

fn history_env(state_root: &std::path::Path) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "XDG_STATE_HOME".to_string(),
        state_root.display().to_string(),
    )])
}
