use std::fs;

use tak_loader::{LoadOptions, load_workspace};

fn write_tasks(root: &std::path::Path, body: &str) {
    fs::create_dir_all(root).expect("create workspace");
    fs::write(root.join("TASKS.py"), body).expect("write TASKS.py");
}

#[test]
fn loader_resolves_named_share_workspace_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = ContainerRuntime(image="alpine:3.20")
SPEC = module_spec(
  sessions=[session("cargo", execution=LocalOnly(Local("local", runtime=RUNTIME)), reuse=ShareWorkspace())],
  tasks=[task("check", steps=[cmd("true")], execution=UseSession("cargo"))],
)
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");

    assert_eq!(spec.sessions.len(), 1);
    assert!(spec.sessions.contains_key("cargo"));
    assert_eq!(
        spec.tasks
            .values()
            .next()
            .expect("task")
            .session
            .as_ref()
            .expect("session ref")
            .name,
        "cargo"
    );
}

#[test]
fn loader_rejects_duplicate_session_names() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = ContainerRuntime(image="alpine:3.20")
SPEC = module_spec(
  sessions=[
    session("cargo", execution=LocalOnly(Local("local", runtime=RUNTIME)), reuse=ShareWorkspace()),
    session("cargo", execution=LocalOnly(Local("local", runtime=RUNTIME)), reuse=ShareWorkspace()),
  ],
  tasks=[],
)
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("duplicate session");

    assert!(
        err.to_string()
            .contains("duplicate session definition: cargo"),
        "{err:#}"
    );
}

#[test]
fn loader_rejects_empty_share_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_tasks(
        temp.path(),
        r#"RUNTIME = ContainerRuntime(image="alpine:3.20")
SPEC = module_spec(
  sessions=[session("cargo", execution=LocalOnly(Local("local", runtime=RUNTIME)), reuse=SharePaths([]))],
  tasks=[],
)
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("empty share paths");

    assert!(
        err.to_string()
            .contains("SharePaths requires at least one path"),
        "{err:#}"
    );
}
