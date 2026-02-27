//! Behavioral tests for loader discovery and module resolution.

use std::fs;

use tak_core::label::parse_label;
use tak_loader::{LoadOptions, detect_workspace_root, discover_tasks_files, load_workspace};

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

/// Ensures dependency lists can reference another task object directly.
#[test]
fn loads_module_with_task_object_dependencies() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=[build], steps=[cmd("echo", "test")])

SPEC = module_spec(tasks=[build, test])
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    let test_task = spec.tasks.get(&test).expect("test task exists");

    assert!(spec.tasks.contains_key(&build));
    assert_eq!(test_task.deps, vec![build]);
}

/// Ensures a single dependency can be passed as a task object (without wrapping in a list).
#[test]
fn loads_module_with_single_task_object_dependency() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");

    let code = r#"
build = task("build", steps=[cmd("echo", "ok")])
test = task("test", deps=build, steps=[cmd("echo", "test")])

SPEC = module_spec(tasks=[build, test])
SPEC
"#;
    fs::write(temp.path().join("apps/web/TASKS.py"), code).expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    let build = parse_label("//apps/web:build", "//").expect("label");
    let test = parse_label("//apps/web:test", "//").expect("label");
    let test_task = spec.tasks.get(&test).expect("test task exists");

    assert_eq!(test_task.deps, vec![build]);
}

/// Ensures workspace root detection ignores legacy `tak.toml` markers and prefers `.git`.
#[test]
fn detect_workspace_root_prefers_git_over_tak_toml() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join(".git")).expect("mkdir git");
    fs::create_dir_all(temp.path().join("workspace/apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("workspace/tak.toml"),
        "project_id = \"legacy\"\n",
    )
    .expect("write tak.toml");

    let start = temp.path().join("workspace/apps/web");
    let root = detect_workspace_root(&start).expect("detect root");
    assert_eq!(
        root,
        temp.path()
            .canonicalize()
            .expect("canonicalize expected root")
    );
}

/// Ensures `module_spec(project_id=...)` in `TASKS.py` defines workspace identity.
#[test]
fn project_id_can_be_defined_in_tasks_module_spec() {
    let temp = tempfile::tempdir().expect("tempdir");

    let root_tasks = r#"
SPEC = module_spec(project_id="tasks-project", tasks=[])
SPEC
"#;
    fs::write(temp.path().join("TASKS.py"), root_tasks).expect("write root tasks");
    fs::create_dir_all(temp.path().join("apps/web")).expect("mkdir");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"
SPEC = module_spec(tasks=[task("build", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write package tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    assert_eq!(spec.project_id, "tasks-project");
}

/// Ensures conflicting module-level project ids fail fast with a clear error.
#[test]
fn rejects_conflicting_module_project_ids() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/a")).expect("mkdir");
    fs::create_dir_all(temp.path().join("apps/b")).expect("mkdir");
    fs::write(
        temp.path().join("apps/a/TASKS.py"),
        r#"
SPEC = module_spec(project_id="a-project", tasks=[task("build", steps=[cmd("echo", "a")])])
SPEC
"#,
    )
    .expect("write a tasks");
    fs::write(
        temp.path().join("apps/b/TASKS.py"),
        r#"
SPEC = module_spec(project_id="b-project", tasks=[task("test", steps=[cmd("echo", "b")])])
SPEC
"#,
    )
    .expect("write b tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("must fail");
    assert!(
        err.to_string().contains("conflicting project_id"),
        "unexpected error: {err}"
    );
}

/// Ensures legacy `tak.toml` no longer controls project id resolution.
#[test]
fn ignores_project_id_from_tak_toml() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("tak.toml"),
        "project_id = \"legacy-config-id\"\n",
    )
    .expect("write tak.toml");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[task("build", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load");
    assert_ne!(spec.project_id, "legacy-config-id");
}
