//! Behavioral tests for loader discovery and module resolution.

use std::fs;

use taskcraft_core::label::parse_label;
use taskcraft_loader::{LoadOptions, discover_tasks_files, load_workspace};

/// Ensures file discovery respects `.gitignore` filtering.
#[test]
fn discovers_tasks_files_respecting_gitignore() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join(".gitignore"), "ignored/\n").expect("write gitignore");

    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::create_dir_all(temp.path().join("ignored/hidden")).expect("mkdir");

    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        "SPEC = {'spec_version': 1}\nSPEC\n",
    )
    .expect("write tasks");
    fs::write(
        temp.path().join("ignored/hidden/TASKS.py"),
        "SPEC = {'spec_version': 1}\nSPEC\n",
    )
    .expect("write ignored tasks");

    let files = discover_tasks_files(temp.path()).expect("discovery");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("apps/web/TASKS.py"));
}

/// Ensures a loaded module yields fully-resolved workspace task labels.
#[test]
fn loads_module_and_resolves_labels() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
SPEC = module_spec(
  tasks=[
    task("build", steps=[cmd("echo", "ok")]),
    task("test", deps=[":build"], steps=[cmd("echo", "test")])
  ]
)
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    assert!(spec.tasks.contains_key(&build));
    assert!(spec.tasks.contains_key(&test));
}
