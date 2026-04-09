use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn workspace_load_uses_only_current_directory_tasks_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let child = temp.path().join("examples/hello");
    fs::create_dir_all(temp.path().join(".git")).expect("git marker");
    fs::create_dir_all(&child).expect("mkdir child");
    fs::write(temp.path().join("TASKS.py"), root_tasks()).expect("write root tasks");
    fs::write(child.join("TASKS.py"), child_tasks()).expect("write child tasks");

    let spec = load_workspace(&child, &LoadOptions::default()).expect("load child workspace");
    let labels = spec.tasks.keys().map(canonical_label).collect::<Vec<_>>();

    assert_eq!(labels, vec!["//:hello"]);
}

#[test]
fn workspace_load_requires_tasks_file_in_current_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let child = temp.path().join("examples/hello");
    fs::create_dir_all(temp.path().join(".git")).expect("git marker");
    fs::create_dir_all(&child).expect("mkdir child");
    fs::write(temp.path().join("TASKS.py"), root_tasks()).expect("write root tasks");

    let err = load_workspace(&child, &LoadOptions::default()).expect_err("load should fail");
    assert!(
        err.to_string().contains("TASKS.py"),
        "error should name TASKS.py: {err:#}"
    );
}

fn root_tasks() -> &'static str {
    r#"SPEC = module_spec(tasks=[task("root_task", steps=[cmd("echo", "root")])])
SPEC
"#
}

fn child_tasks() -> &'static str {
    r#"SPEC = module_spec(tasks=[task("hello", steps=[cmd("echo", "hello")])])
SPEC
"#
}

fn canonical_label(label: &tak_core::model::TaskLabel) -> String {
    match label.package.as_str() {
        "//" => format!("//:{}", label.name),
        _ => format!("{}:{}", label.package, label.name),
    }
}
