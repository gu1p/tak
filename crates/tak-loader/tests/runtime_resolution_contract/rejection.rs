use tak_loader::{LoadOptions, load_workspace};

use super::support::write_root_and_app_tasks;

#[test]
fn rejects_dockerfile_outside_resolved_build_context() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(runtime=Runtime.Dockerfile(path("../Dockerfile")))
SPEC = module_spec(tasks=[task("remote_only", steps=[cmd("echo", "ok")], execution=REMOTE)])
SPEC
"#,
    );

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    assert!(
        err.to_string()
            .contains("execution Remote.runtime.dockerfile must be within build_context"),
        "unexpected error: {err:#}"
    );
}
