use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn explicit_includes_load_child_modules() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("apps/web");
    fs::create_dir_all(&app_dir).expect("mkdir child");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(
  includes=[path("apps/web")],
  tasks=[task("all", deps=["//apps/web:test"], steps=[cmd("echo", "root")])],
)
SPEC
"#,
    )
    .expect("write root tasks");
    fs::write(
        app_dir.join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[task("test", steps=[cmd("echo", "child")])])
SPEC
"#,
    )
    .expect("write child tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let labels = spec
        .tasks
        .keys()
        .map(|label| {
            if label.package == "//" {
                format!("//:{}", label.name)
            } else {
                format!("{}:{}", label.package, label.name)
            }
        })
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["//:all", "//apps/web:test"]);
}

#[test]
fn include_cycles_fail_with_path_diagnostics() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("apps/web");
    fs::create_dir_all(&app_dir).expect("mkdir child");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(
  includes=[path("apps/web")],
  tasks=[],
)
SPEC
"#,
    )
    .expect("write root tasks");
    fs::write(
        app_dir.join("TASKS.py"),
        r#"SPEC = module_spec(
  includes=[path("../../")],
  tasks=[task("test", steps=[cmd("echo", "child")])],
)
SPEC
"#,
    )
    .expect("write child tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("include cycle"),
        "missing cycle error: {rendered}"
    );
    assert!(
        rendered.contains("apps/web"),
        "missing include path context in cycle error: {rendered}"
    );
}
