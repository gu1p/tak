use std::fs;

use tak_loader::{LoadOptions, load_workspace};

const POLICY_AS_SESSION: &str = r#"POLICY: ExecutionPolicySpec = execution_policy(placements=[Execution.Local()])
SPEC = module_spec(tasks=[
  task("bad", steps=[cmd("true")], execution=Execution.Session(POLICY)),
])
SPEC
"#;

#[test]
fn session_spec_stub_keeps_session_fields_required() {
    let stubs = include_str!("../src/loader/dsl_stubs.pyi");

    assert!(stubs.contains("class SessionSpec(TypedDict):"));
    assert!(!stubs.contains("class SessionSpec(TypedDict, total=False):"));
}

#[test]
fn runtime_rejects_execution_policy_as_session_object() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), POLICY_AS_SESSION).expect("write TASKS.py");

    let options = LoadOptions {
        enable_type_check: false,
        ..LoadOptions::default()
    };
    let err = load_workspace(temp.path(), &options).expect_err("runtime rejection");
    let message = err.to_string();

    assert!(
        message.contains("Execution.Session(...) expects a session(...) object"),
        "{err:#}"
    );
}
