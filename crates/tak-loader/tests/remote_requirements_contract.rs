use std::fs;

use tak_core::model::{
    ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RemoteTransportKind, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn resolves_requirements_based_remote_execution() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app_dir = temp.path().join("apps/web");
    fs::create_dir_all(&app_dir).expect("mkdir");
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
        app_dir.join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[
  task("remote_only", steps=[cmd("echo", "ok")], execution=Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=Transport.TorOnionService(),
    runtime=Runtime.Image("alpine:3.20"),
  )),
])
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec
        .tasks
        .values()
        .next()
        .expect("resolved task should exist");
    match &task.execution {
        TaskExecutionSpec::RemoteOnly(remote) => {
            assert_eq!(remote.pool.as_deref(), Some("build"));
            assert_eq!(remote.required_tags, vec!["builder"]);
            assert_eq!(remote.required_capabilities, vec!["linux"]);
            assert_eq!(remote.transport_kind, RemoteTransportKind::Tor);
            assert!(matches!(
                remote.runtime,
                Some(RemoteRuntimeSpec::Containerized {
                    source: ContainerRuntimeSourceSpec::Image { ref image }
                }) if image == "alpine:3.20"
            ));
        }
        other => panic!("expected requirements-based remote execution, got {other:?}"),
    }
}

#[test]
fn rejects_removed_task_side_remote_identity_and_endpoint() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
SPEC = module_spec(tasks=[task("removed-remote-fields", steps=[cmd("echo", "ok")], execution=Execution.Remote(id="builder-a", endpoint="http://127.0.0.1:43123"))])
SPEC
"#,
    )
    .expect("write tasks");

    let err = load_workspace(temp.path(), &LoadOptions::default()).expect_err("load should fail");
    assert!(
        err.to_string().contains("unexpected keyword argument 'id'"),
        "removed endpoint/id should be rejected explicitly: {err:#}"
    );
}
