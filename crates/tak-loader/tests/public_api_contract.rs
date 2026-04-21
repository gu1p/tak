use std::fs;

use tak_loader::{LoadOptions, detect_workspace_root, discover_tasks_files};

#[test]
fn crate_root_exports_workspace_discovery_api() {
    let temp = tempfile::tempdir().expect("tempdir");
    let child = temp.path().join("apps/web");
    fs::create_dir_all(&child).expect("mkdir child");
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
        child.join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[task("test", steps=[cmd("echo", "child")])])
SPEC
"#,
    )
    .expect("write child tasks");

    let detected = detect_workspace_root(temp.path()).expect("detect workspace root");
    assert_eq!(
        detected,
        temp.path().canonicalize().expect("canonicalize tempdir")
    );

    let discovered =
        discover_tasks_files(temp.path(), &LoadOptions::default()).expect("discover tasks files");
    let relative_paths = discovered
        .iter()
        .map(|(path, _)| {
            path.strip_prefix(&detected)
                .expect("path under workspace root")
                .display()
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(relative_paths, vec!["TASKS.py", "apps/web/TASKS.py"]);
}
