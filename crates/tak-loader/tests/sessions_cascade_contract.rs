use std::fs;

use tak_core::model::TaskExecutionSpec;
use tak_loader::{LoadOptions, load_workspace};

fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root).expect("create workspace");
    fs::write(root.join("TASKS.py"), body).expect("write TASKS.py");
}

#[test]
fn loader_preserves_use_session_cascade_flag() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SPEC = module_spec(
  tasks=[task("check", steps=[cmd("true")], execution=Execution.Session(SESSION, cascade=True))],
)
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");

    match &task.execution {
        TaskExecutionSpec::UseSession { name, cascade } => {
            assert!(name.starts_with("__tak_session_"), "session name: {name}");
            assert!(*cascade);
        }
        other => panic!("unexpected execution: {other:?}"),
    }
}

#[test]
fn loader_defaults_use_session_cascade_to_false() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SPEC = module_spec(
  tasks=[task("check", steps=[cmd("true")], execution=Execution.Session(SESSION))],
)
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");

    match &task.execution {
        TaskExecutionSpec::UseSession { name, cascade } => {
            assert!(name.starts_with("__tak_session_"), "session name: {name}");
            assert!(!*cascade);
        }
        other => panic!("unexpected execution: {other:?}"),
    }
}
