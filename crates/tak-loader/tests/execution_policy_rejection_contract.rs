use std::fs;

use tak_loader::{LoadOptions, load_workspace};

fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root).expect("create workspace");
    fs::write(root.join("TASKS.py"), body).expect("write TASKS.py");
}

#[test]
fn task_rejects_mixed_execution_and_execution_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"POLICY = execution_policy("local", [Execution.Local()])
SPEC = module_spec(
  execution_policies=[POLICY],
  tasks=[
    task("bad", steps=[cmd("true")], execution=Execution.Local(), execution_policy="local")
  ],
)
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("mixed execution");
    assert!(
        err.to_string()
            .contains("task `bad` cannot set both execution and execution_policy"),
        "{err:#}"
    );
}

#[test]
fn session_rejects_mixed_execution_and_execution_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
POLICY = execution_policy("local", [Execution.Local(runtime=RUNTIME)])
SPEC = module_spec(
  execution_policies=[POLICY],
  sessions=[
    session(
      "bad",
      execution=Execution.Local(runtime=RUNTIME),
      execution_policy="local",
      reuse=SessionReuse.Workspace(),
    )
  ],
  tasks=[],
)
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("mixed session");
    assert!(
        err.to_string()
            .contains("session `bad` cannot set both execution and execution_policy"),
        "{err:#}"
    );
}
