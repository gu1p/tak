use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn attached_execution_sessions_reject_mismatched_implicit_contexts() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SESSION = session("shared", reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[
  task(
    "frontend",
    context=CurrentState(roots=[path("frontend")]),
    steps=[cmd("true")],
    execution=Execution.Local(session=SESSION),
  ),
  task(
    "backend",
    context=CurrentState(roots=[path("backend")]),
    steps=[cmd("true")],
    execution=Execution.Local(session=SESSION),
  ),
])
SPEC
"#,
    )
    .expect("write TASKS.py");

    let err = load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("load should fail")
        .to_string();

    assert!(
        err.contains("does not match the first CurrentState"),
        "{err:#}"
    );
}
