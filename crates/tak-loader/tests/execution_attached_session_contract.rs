use std::fs;

use tak_core::model::{ExecutionPlacementSpec, TaskExecutionSpec, WorkspaceSpec};
use tak_loader::{LoadOptions, load_workspace};

fn load_spec(body: &str) -> WorkspaceSpec {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(temp.path().join("TASKS.py"), body).expect("write TASKS.py");
    load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace")
}

fn load_error(body: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(temp.path().join("TASKS.py"), body).expect("write TASKS.py");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("load should fail")
        .to_string()
}

#[test]
fn local_execution_accepts_attached_session_without_container() {
    let spec = load_spec(
        r#"SESSION = session("state", reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[
  task("check", steps=[cmd("true")], execution=Execution.Local(session=SESSION)),
])
SPEC
"#,
    );
    let task = spec.tasks.values().next().expect("task");
    let TaskExecutionSpec::LocalOnly(local) = &task.execution else {
        panic!("unexpected execution: {:?}", task.execution);
    };
    assert_eq!(
        local.session.as_ref().expect("session").display_name,
        "state"
    );
}

#[test]
fn first_available_accepts_mixed_session_and_plain_local_placements() {
    let spec = load_spec(
        r#"CONTAINER = Container.Image("alpine:3.20")
SESSION = session("remote-state", reuse=SessionReuse.Workspace())
EXEC = Execution.FirstAvailable([
  Execution.Remote(container=CONTAINER, session=SESSION),
  Execution.Local(),
])
SPEC = module_spec(tasks=[
  task("check", steps=[cmd("true")], execution=EXEC, cascade_execution=True),
])
SPEC
"#,
    );
    let task = spec.tasks.values().next().expect("task");
    let TaskExecutionSpec::ByExecutionPolicy { placements, .. } = &task.execution else {
        panic!("unexpected execution: {:?}", task.execution);
    };
    assert!(
        matches!(&placements[0], ExecutionPlacementSpec::Remote(remote) if remote.session.is_some())
    );
    assert!(
        matches!(&placements[1], ExecutionPlacementSpec::Local(local) if local.session.is_none())
    );
}

#[test]
fn remote_execution_with_session_requires_runtime_or_default_container() {
    let err = load_error(
        r#"SESSION = session("remote-state", reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[
  task("check", steps=[cmd("true")], execution=Execution.Remote(session=SESSION)),
])
SPEC
"#,
    );
    assert!(
        err.contains("Execution.Remote(session=...) requires a container"),
        "{err:#}"
    );
}
