use crate::support;

use std::collections::BTreeMap;
use support::run_tak_output;

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
