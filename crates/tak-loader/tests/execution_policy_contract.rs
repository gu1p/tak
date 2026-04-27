use tak_core::model::{
    ContainerRuntimeSourceSpec, ExecutionPlacementSpec, RemoteRuntimeSpec, RemoteSelectionSpec,
    TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

use std::fs;

fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root).expect("create workspace");
    fs::write(root.join("TASKS.py"), body).expect("write TASKS.py");
}
#[test]
fn module_default_execution_policy_resolves_ordered_placements() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
POLICY = execution_policy(
  "remote-then-local",
  [
    Execution.Remote(pool="build", required_tags=["builder"], runtime=RUNTIME),
    Execution.Local(runtime=RUNTIME),
  ],
)
SPEC = module_spec(
  execution_policies=[POLICY],
  defaults={"execution_policy": "remote-then-local"},
  tasks=[task("check", steps=[cmd("true")])],
)
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");

    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => {
            assert_eq!(name, "remote-then-local");
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
        other => panic!("expected named execution policy, got {other:?}"),
    }
}

#[test]
fn execution_policy_accepts_explicit_shuffle_remote_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
POLICY = execution_policy(
  "spread",
  [Execution.Remote(runtime=RUNTIME, selection=RemoteSelection.Shuffle())],
)
SPEC = module_spec(
  execution_policies=[POLICY],
  tasks=[task("check", steps=[cmd("true")], execution_policy="spread")],
)
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");
    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => match &placements[0] {
            ExecutionPlacementSpec::Remote(remote) => {
                assert!(matches!(remote.selection, RemoteSelectionSpec::Shuffle));
            }
            other => panic!("expected remote placement, got {other:?}"),
        },
        other => panic!("expected named execution policy, got {other:?}"),
    }
}

fn assert_runtime_image(runtime: Option<&RemoteRuntimeSpec>, expected: &str) {
    match runtime.expect("runtime") {
        RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
        } => assert_eq!(image, expected),
        other => panic!("expected image runtime, got {other:?}"),
    }
}
