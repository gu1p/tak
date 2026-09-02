use std::fs;

use tak_loader::{LoadOptions, detect_workspace_root, inspect_authored_root_module};

#[test]
fn crate_root_exports_v2_workspace_discovery_api() {
    let temp = tempfile::tempdir().expect("tempdir");
    let child = temp.path().join("apps/web");
    fs::create_dir_all(&child).expect("mkdir child");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(
  spec_version=2,
  includes=[path("apps/web")],
  tasks=[],
)
SPEC
"#,
    )
    .expect("write root tasks");
    fs::write(
        child.join("TASKS.py"),
        r#"SPEC = module_spec(spec_version=2, tasks=[task("test", steps=[cmd("echo", "child")])])
SPEC
"#,
    )
    .expect("write child tasks");

    let detected = detect_workspace_root(temp.path()).expect("detect workspace root");
    assert_eq!(
        detected,
        temp.path().canonicalize().expect("canonicalize tempdir")
    );

    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .expect("inspect v2 workspace");
    assert_eq!(root.module.tasks[0].name, "//apps/web:test");
}
