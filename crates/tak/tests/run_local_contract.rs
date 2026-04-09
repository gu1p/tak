//! Contract test for `tak run` staying client-side.

use std::fs;
use std::process::Command as StdCommand;

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

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
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
