use std::fs;

use tak_core::model::OutputSelectorSpec;
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn task_outputs_resolve_to_workspace_relative_paths_and_globs() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("apps/web")).expect("create package");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(includes=[path("apps/web")], tasks=[])
SPEC
"#,
    )
    .expect("write root tasks");
    fs::write(
        temp.path().join("apps/web/TASKS.py"),
        r#"SPEC = module_spec(tasks=[task(
  "build",
  outputs=[path("dist"), glob("reports/**")],
  steps=[cmd("echo", "ok")],
)])
SPEC
"#,
    )
    .expect("write package tasks");

    let workspace = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = workspace
        .tasks
        .get(&tak_core::label::parse_label("//apps/web:build", "//").expect("label"))
        .expect("task");

    assert_eq!(
        task.outputs,
        vec![
            OutputSelectorSpec::Path(
                tak_core::model::normalize_path_ref("workspace", "apps/web/dist")
                    .expect("dist path"),
            ),
            OutputSelectorSpec::Glob {
                pattern: "apps/web/reports/**".into(),
            },
        ]
    );
}

#[test]
fn task_output_globs_cannot_escape_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(tasks=[task(
  "check",
  outputs=[glob("../out/**")],
  steps=[cmd("echo", "ok")],
)])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("glob should fail");
    assert!(
        err.to_string().contains("output glob escapes workspace"),
        "unexpected error: {err:#}"
    );
}
