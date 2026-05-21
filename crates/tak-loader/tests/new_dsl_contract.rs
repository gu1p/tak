use std::fs;

use tak_core::model::{ExecutionPlacementSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

fn load_task(source: &str) -> tak_core::model::ResolvedTask {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect("load workspace")
        .tasks
        .into_values()
        .next()
        .expect("task")
}

fn load_error(source: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("workspace should fail")
        .to_string()
}

#[test]
fn first_available_resolves_ordered_container_placements() {
    let task = load_task(
        r#"CONTAINER = Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512))
EXEC = Execution.FirstAvailable([
  Execution.Remote(pool="build", required_tags=["builder"], container=CONTAINER),
  Execution.Local(container=CONTAINER),
])
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], execution=EXEC)])
SPEC
"#,
    );

    let TaskExecutionSpec::ByExecutionPolicy { placements, .. } = task.execution else {
        panic!("expected FirstAvailable policy");
    };
    assert!(matches!(placements[0], ExecutionPlacementSpec::Remote(_)));
    assert!(matches!(placements[1], ExecutionPlacementSpec::Local(_)));
}

#[test]
fn task_use_session_with_cascade_replaces_execution_session_wrapper() {
    let task = load_task(
        r#"CONTAINER = Container.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(container=CONTAINER), reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], use_session=SESSION, cascade_session=True)])
SPEC
"#,
    );

    let TaskExecutionSpec::UseSession { cascade, .. } = task.execution else {
        panic!("expected use_session task execution");
    };
    assert!(cascade);
    assert!(task.session.is_some());
}

#[test]
fn task_rejects_invalid_session_arguments() {
    let missing_session = load_error(
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], cascade_session=True)])
SPEC
"#,
    );
    assert!(
        missing_session.contains("cascade_session=True requires use_session"),
        "{missing_session:#}"
    );

    let mixed_execution = load_error(
        r#"CONTAINER = Container.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(container=CONTAINER), reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], execution=Execution.Local(), use_session=SESSION)])
SPEC
"#,
    );
    assert!(
        mixed_execution.contains("cannot use both execution and use_session"),
        "{mixed_execution:#}"
    );
}
