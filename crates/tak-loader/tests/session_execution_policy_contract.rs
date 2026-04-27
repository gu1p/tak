use std::fs;

use tak_core::model::{SessionReuseSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn sessions_can_use_named_execution_policies_with_workspace_and_paths_reuse() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
POLICY = execution_policy("session-local", [Execution.Local(runtime=RUNTIME)])
SPEC = module_spec(
  execution_policies=[POLICY],
  sessions=[
    session("workspace", execution_policy="session-local", reuse=SessionReuse.Workspace()),
    session("paths", execution_policy="session-local", reuse=SessionReuse.Paths([path("out")])),
  ],
  tasks=[
    task("a", steps=[cmd("true")], execution=Execution.Session("workspace")),
    task("b", steps=[cmd("true")], execution=Execution.Session("paths")),
  ],
)
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let workspace = spec.sessions.get("workspace").expect("workspace session");
    let paths = spec.sessions.get("paths").expect("paths session");

    assert_session_policy(&workspace.execution);
    assert_session_policy(&paths.execution);
    assert!(matches!(workspace.reuse, SessionReuseSpec::ShareWorkspace));
    assert!(matches!(paths.reuse, SessionReuseSpec::SharePaths { .. }));
}

fn assert_session_policy(execution: &TaskExecutionSpec) {
    match execution {
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => {
            assert_eq!(name, "session-local");
            assert_eq!(placements.len(), 1);
        }
        other => panic!("expected session execution policy, got {other:?}"),
    }
}
