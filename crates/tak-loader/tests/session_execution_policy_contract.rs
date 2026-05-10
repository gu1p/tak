use std::fs;

use tak_core::model::{SessionReuseSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn sessions_can_use_execution_policy_objects_with_workspace_and_paths_reuse() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"RUNTIME = Container.Image("alpine:3.20")
POLICY = Execution.FirstAvailable(placements=[Execution.Local(container=RUNTIME)])
WORKSPACE_SESSION = session("workspace", execution=POLICY, reuse=SessionReuse.Workspace())
PATHS_SESSION = session("paths", execution=POLICY, reuse=SessionReuse.Paths([path("out")]))
SPEC = module_spec(
  tasks=[
    task("a", steps=[cmd("true")], use_session=WORKSPACE_SESSION),
    task("b", steps=[cmd("true")], use_session=PATHS_SESSION),
  ],
)
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let mut sessions = spec.sessions.values();
    let first = sessions.next().expect("first session");
    let second = sessions.next().expect("second session");
    let (workspace, paths) = match (&first.reuse, &second.reuse) {
        (SessionReuseSpec::ShareWorkspace, SessionReuseSpec::SharePaths { .. }) => (first, second),
        (SessionReuseSpec::SharePaths { .. }, SessionReuseSpec::ShareWorkspace) => (second, first),
        other => panic!("expected workspace and paths sessions, got {other:?}"),
    };

    assert_session_policy(
        workspace
            .execution
            .as_deref()
            .expect("workspace legacy execution"),
    );
    assert_session_policy(paths.execution.as_deref().expect("paths legacy execution"));
    assert!(matches!(workspace.reuse, SessionReuseSpec::ShareWorkspace));
    assert!(matches!(paths.reuse, SessionReuseSpec::SharePaths { .. }));
}

fn assert_session_policy(execution: &TaskExecutionSpec) {
    match execution {
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => {
            assert!(name.starts_with("__tak_policy_"), "policy name: {name}");
            assert_eq!(placements.len(), 1);
        }
        other => panic!("expected session execution policy, got {other:?}"),
    }
}

#[test]
fn loader_resolves_container_reuse_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"RUNTIME = Container.Image("alpine:3.20")
SESSION = session("container", execution=Execution.Local(container=RUNTIME), reuse=SessionReuse.Container())
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], use_session=SESSION)])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let session = spec.sessions.values().next().expect("session");
    assert!(matches!(session.reuse, SessionReuseSpec::Container));
}
