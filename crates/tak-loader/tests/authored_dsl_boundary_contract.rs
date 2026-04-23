use std::fs;

use tak_loader::{LoadOptions, evaluate_named_policy_decision, load_workspace};

#[test]
fn rejects_tak_imports_with_explicit_migration_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"from tak import module_spec, task, cmd

SPEC = module_spec(tasks=[task("check", steps=[cmd("echo", "ok")])])
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
        "missing import migration guidance: {message:#}"
    );
}

#[test]
fn rejects_legacy_remote_transport_namespace_with_explicit_migration_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
REMOTE = Remote(pool="build", transport=RemoteTransportMode.TorOnionService())
SPEC = module_spec(tasks=[task("check", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE))])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    let message = err.to_string();
    assert!(
        message.contains("`RemoteTransportMode.TorOnionService(...)` is unsupported"),
        "missing legacy transport rejection: {message:#}"
    );
    assert!(
        message.contains("use `TorOnionService()` instead"),
        "missing transport migration guidance: {message:#}"
    );
}

#[test]
fn rejects_legacy_decision_remote_any_with_explicit_migration_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tasks_file = temp.path().join("TASKS.py");
    fs::write(
        &tasks_file,
        r#"def choose_remote(ctx):
  return Decision.remote_any([Remote(pool="build")], reason=Reason.LOCAL_CPU_HIGH)
"#,
    )
    .expect("write tasks");

    let err = evaluate_named_policy_decision(&tasks_file, "//", "choose_remote")
        .expect_err("eval should fail");
    let message = err.to_string();
    assert!(
        message.contains("`Decision.remote_any(...)` is unsupported"),
        "missing remote_any rejection: {message:#}"
    );
    assert!(
        message.contains("use `Decision.remote(...)`"),
        "missing remote_any migration guidance: {message:#}"
    );
}
