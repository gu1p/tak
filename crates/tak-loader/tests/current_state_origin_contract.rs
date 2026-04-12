use std::fs;

use tak_core::model::CurrentStateOrigin;
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn omitted_context_resolves_to_implicit_default_origin() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");

    assert_eq!(task.context.origin, CurrentStateOrigin::ImplicitDefault);
}

#[test]
fn explicit_context_resolves_to_explicit_origin() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[task(
  "check",
  context=CurrentState(),
  steps=[cmd("echo", "ok")],
)])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec.tasks.values().next().expect("task");

    assert_eq!(task.context.origin, CurrentStateOrigin::Explicit);
}
