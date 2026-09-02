use std::fs;

use serde_json::json;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn v2_loader_preserves_explicit_per_task_context_rules() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "check",
  context=CurrentState(
    roots=[path("src")],
    ignored=[path("src/generated"), gitignore()],
    include=[path("src/generated/keep.txt")],
  ),
)])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let task = serde_json::to_value(&root.module.tasks[0]).unwrap();
    assert_eq!(
        task["context"],
        json!({
            "roots": ["src"],
            "ignored_paths": ["src/generated"],
            "use_gitignore": true,
            "include": ["src/generated/keep.txt"]
        })
    );
}

#[test]
fn v2_loader_preserves_session_context_for_inheriting_tasks() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"S = session("build", execution=Execution.Local(),
  context=CurrentState(roots=[path("crates")]))
SPEC = module_spec(spec_version=2, tasks=[task("check", use_session=S)])
SPEC
"#,
    )
    .unwrap();

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let context = serde_json::to_value(
        root.module.tasks[0]
            .session
            .as_ref()
            .unwrap()
            .context
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(context["roots"], json!(["crates"]));
}
