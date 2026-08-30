use std::fs;

use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn default_shared_workspace_execution_cannot_be_weakened_by_a_task() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SHARED = session(
  "build",
  reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
  affinity=Affinity.RequireSameNode("build"),
)
SPEC = module_spec(
  spec_version=2,
  defaults=Defaults(execution=Execution.Remote(session=SHARED)),
  tasks=[task("check", affinity=Affinity.PreferSameNode("build"))],
)
SPEC
"#,
    )
    .expect("write tasks");

    let error = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .expect_err("task affinity must not weaken its default shared session")
        .to_string();
    assert!(error.contains("cannot weaken or change"), "{error}");
}
