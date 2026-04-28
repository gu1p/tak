use std::fs;

use tak_core::model::{
    ContainerRuntimeSourceSpec, ExecutionPlacementSpec, RemoteRuntimeSpec, RemoteSelectionSpec,
    ResolvedTask, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

fn load_task(body: &str) -> ResolvedTask {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(temp.path().join("TASKS.py"), body).expect("write TASKS.py");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect("load workspace")
        .tasks
        .into_values()
        .next()
        .expect("task")
}

fn policy_placements(task: &ResolvedTask) -> &[ExecutionPlacementSpec] {
    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => placements,
        other => panic!("expected inline execution policy, got {other:?}"),
    }
}

#[test]
fn inline_execution_policy_on_task_resolves_ordered_placements() {
    let task = load_task(
        r#"RUNTIME=Runtime.Image("alpine:3.20"); POLICY=execution_policy(placements=[Execution.Remote(pool="build", required_tags=["builder"], runtime=RUNTIME), Execution.Local(runtime=RUNTIME)]); SPEC=module_spec(tasks=[task("check", steps=[cmd("true")], execution=POLICY)]); SPEC"#,
    );

    let placements = policy_placements(&task);
    assert_eq!(placements.len(), 2);
    match &placements[0] {
        ExecutionPlacementSpec::Remote(remote) => {
            assert_eq!(remote.pool.as_deref(), Some("build"));
            assert_eq!(remote.required_tags, vec!["builder"]);
            assert!(matches!(remote.selection, RemoteSelectionSpec::Sequential));
            assert_runtime_image(remote.runtime.as_ref(), "alpine:3.20");
        }
        other => panic!("expected first remote placement, got {other:?}"),
    }
    match &placements[1] {
        ExecutionPlacementSpec::Local(local) => {
            assert_runtime_image(local.runtime.as_ref(), "alpine:3.20");
        }
        other => panic!("expected second local placement, got {other:?}"),
    }
}

#[test]
fn module_default_execution_accepts_inline_execution_policy() {
    let task = load_task(
        r#"RUNTIME=Runtime.Image("alpine:3.20"); POLICY=execution_policy(placements=[Execution.Remote(pool="build", runtime=RUNTIME)]); SPEC=module_spec(defaults=Defaults(execution=POLICY), tasks=[task("check", steps=[cmd("true")])]); SPEC"#,
    );
    match &policy_placements(&task)[0] {
        ExecutionPlacementSpec::Remote(remote) => {
            assert_eq!(remote.pool.as_deref(), Some("build"));
            assert_runtime_image(remote.runtime.as_ref(), "alpine:3.20");
        }
        other => panic!("expected remote placement, got {other:?}"),
    }
}

#[test]
fn module_default_execution_accepts_direct_execution_value() {
    let task = load_task(
        r#"RUNTIME=Runtime.Image("alpine:3.20"); SPEC=module_spec(defaults=Defaults(execution=Execution.Local(runtime=RUNTIME)), tasks=[task("check", steps=[cmd("true")])]); SPEC"#,
    );
    match &task.execution {
        TaskExecutionSpec::LocalOnly(local) => {
            assert_runtime_image(local.runtime.as_ref(), "alpine:3.20")
        }
        other => panic!("expected local default execution, got {other:?}"),
    }
}

#[test]
fn execution_policy_accepts_explicit_shuffle_remote_selection() {
    let task = load_task(
        r#"RUNTIME=Runtime.Image("alpine:3.20"); POLICY=execution_policy(placements=[Execution.Remote(runtime=RUNTIME, selection=RemoteSelection.Shuffle())]); SPEC=module_spec(tasks=[task("check", steps=[cmd("true")], execution=POLICY)]); SPEC"#,
    );
    match &policy_placements(&task)[0] {
        ExecutionPlacementSpec::Remote(remote) => {
            assert!(matches!(remote.selection, RemoteSelectionSpec::Shuffle))
        }
        other => panic!("expected remote placement, got {other:?}"),
    }
}

fn assert_runtime_image(runtime: Option<&RemoteRuntimeSpec>, expected: &str) {
    let RemoteRuntimeSpec::Containerized { source } = runtime.expect("runtime");
    let ContainerRuntimeSourceSpec::Image { image } = source else {
        panic!("expected image runtime");
    };
    assert_eq!(image, expected);
}
