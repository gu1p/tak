use crate::support;

use std::fs;
use std::process::Command as StdCommand;

#[test]
fn task_logs_reads_output_from_locally_initiated_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(
        workspace.join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("history", steps=[cmd("sh", "-c", "printf 'history stdout\n'; printf 'history stderr\n' >&2")]),
])
SPEC
"#,
    )
    .expect("write tasks");

    let run = StdCommand::new(support::tak_bin())
        .args(["run", "//:history"])
        .current_dir(&workspace)
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak");
    assert!(
        run.status.success(),
        "tak run should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let run_stdout = String::from_utf8_lossy(&run.stdout);
    let task_run_id = extract_task_run_id(&run_stdout);

    let list = StdCommand::new(support::tak_bin())
        .args(["task", "list"])
        .current_dir(&workspace)
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak task list");
    assert!(
        list.status.success(),
        "tak task list should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&list.stdout),
        String::from_utf8_lossy(&list.stderr)
    );
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(
        list_stdout.contains(&format!("task_run_id={task_run_id}")),
        "missing task id:\n{list_stdout}"
    );
    assert!(
        list_stdout.contains("task_label=//:history"),
        "missing task label:\n{list_stdout}"
    );

    let logs = StdCommand::new(support::tak_bin())
        .args(["task", "logs", &task_run_id])
        .current_dir(&workspace)
        .env("XDG_STATE_HOME", &state_root)
        .output()
        .expect("run tak task logs");
    assert!(
        logs.status.success(),
        "tak task logs should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&logs.stdout),
        String::from_utf8_lossy(&logs.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&logs.stdout), "history stdout\n");
    assert_eq!(String::from_utf8_lossy(&logs.stderr), "history stderr\n");
}

fn extract_task_run_id(output: &str) -> String {
    output
        .split_whitespace()
        .find_map(|word| word.trim_start_matches('(').strip_prefix("task_run_id="))
        .map(|value| value.trim_end_matches([',', ')']).to_string())
        .unwrap_or_else(|| panic!("missing task_run_id in output:\n{output}"))
}
