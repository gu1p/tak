use std::fs;

use tak_core::model::WorkspaceSpec;
use tak_loader::{LoadOptions, load_workspace};

fn load_spec(body: &str) -> WorkspaceSpec {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(temp.path().join("TASKS.py"), body).expect("write TASKS.py");
    load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace")
}

fn only_task_session_name(spec: &WorkspaceSpec) -> &str {
    &spec
        .tasks
        .values()
        .next()
        .expect("task")
        .session
        .as_ref()
        .expect("session ref")
        .name
}

#[test]
fn loader_resolves_object_share_workspace_session_without_module_registry() {
    let spec = load_spec(
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], execution=Execution.Session(SESSION))])
SPEC
"#,
    );

    assert_eq!(spec.sessions.len(), 1);
    let task_session = only_task_session_name(&spec);
    assert!(
        spec.sessions.contains_key(task_session),
        "session {task_session} should be registered"
    );
}

#[test]
fn loader_allows_duplicate_diagnostic_session_names_when_objects_are_distinct() {
    let spec = load_spec(
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION_A = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SESSION_B = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[
  task("a", steps=[cmd("true")], execution=Execution.Session(SESSION_A)),
  task("b", steps=[cmd("true")], execution=Execution.Session(SESSION_B)),
])
SPEC
"#,
    );
    assert_eq!(spec.sessions.len(), 2);
}

#[test]
fn module_default_execution_accepts_session_value() {
    let spec = load_spec(
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())
SPEC = module_spec(defaults=Defaults(execution=Execution.Session(SESSION)), tasks=[task("check", steps=[cmd("true")])])
SPEC
"#,
    );
    let task_session = only_task_session_name(&spec);
    assert!(
        spec.sessions.contains_key(task_session),
        "session {task_session} should be registered"
    );
}

#[test]
fn loader_rejects_empty_share_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path()).expect("create workspace");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("cargo", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Paths([]))
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")], execution=Execution.Session(SESSION))])
SPEC
"#,
    )
    .expect("write TASKS.py");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("empty share paths");
    assert!(
        err.to_string()
            .contains("SessionReuse.Paths requires at least one path"),
        "{err:#}"
    );
}
