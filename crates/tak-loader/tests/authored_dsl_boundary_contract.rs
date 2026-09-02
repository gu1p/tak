use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn rejects_tak_imports_with_direct_dsl_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"from tak import module_spec, task, cmd

SPEC = module_spec(spec_version=2, tasks=[task("check", steps=[cmd("echo", "ok")])])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    let message = err.to_string();
    assert!(
        message.contains("imports from `tak` are unsupported"),
        "missing import rejection: {message:#}"
    );
    assert!(
        message.contains("use the shipped TASKS.py DSL directly"),
        "missing import direct DSL guidance: {message:#}"
    );
}

#[test]
fn removed_container_command_has_actionable_authored_diagnostic() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(spec_version=2, tasks=[task("check",
  execution=Execution.Local(container=Container.Image(
    "alpine:3.20", command=["sh", "-c", "true"])))])
SPEC
"#,
    )
    .expect("write tasks");

    let options = LoadOptions {
        enable_type_check: false,
        ..LoadOptions::default()
    };
    let error = load_workspace(temp.path(), &options)
        .expect_err("removed command must fail")
        .to_string();
    assert!(error.contains("Container `command` was removed"), "{error}");
    assert!(error.contains("spec_version=2"), "{error}");
    assert!(error.contains("use task steps"), "{error}");
}
