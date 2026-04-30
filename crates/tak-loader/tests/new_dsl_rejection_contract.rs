use std::fs;

use tak_loader::{LoadOptions, load_workspace};

fn load_error(source: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");
    load_workspace(temp.path(), &LoadOptions::default())
        .expect_err("workspace should fail")
        .to_string()
}

#[test]
fn old_runtime_and_session_policy_surface_is_rejected() {
    let runtime = load_error(
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SPEC = module_spec(tasks=[task("check", steps=[cmd("true")])])
SPEC
"#,
    );
    assert!(
        runtime.contains("use `Container.Image(...)`"),
        "{runtime:#}"
    );

    let host_runtime = load_error(
        r#"RUNTIME = Runtime.Host()
SPEC = module_spec(tasks=[task("check", execution=Execution.Local(runtime=RUNTIME))])
SPEC
"#,
    );
    assert!(
        host_runtime.contains("omit the container") && host_runtime.contains("Execution.Local()"),
        "{host_runtime:#}"
    );
    assert!(!host_runtime.contains("Container.Host"), "{host_runtime:#}");

    let session_wrapper = load_error(
        r#"SESSION = session("cargo", execution=Execution.Local(), reuse=SessionReuse.Workspace())
SPEC = module_spec(tasks=[task("check", execution=Execution.Session(SESSION))])
SPEC
"#,
    );
    assert!(
        session_wrapper.contains("use `task(..., use_session=SESSION)`"),
        "{session_wrapper:#}"
    );
    assert!(
        !session_wrapper.contains("cascade_session=True"),
        "{session_wrapper:#}"
    );
}
