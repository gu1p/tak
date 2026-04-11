use std::fs;

use tak_core::model::{
    ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RemoteTransportKind, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn omitted_transport_resolves_to_any_transport() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
REMOTE = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  runtime=ContainerRuntime(image="alpine:3.20"),
)

SPEC = module_spec(tasks=[
  task("remote_only", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
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
            assert_eq!(remote.transport_kind, RemoteTransportKind::Any);
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
fn any_transport_helper_resolves_to_any_transport() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
REMOTE = Remote(
  pool="build",
  transport=AnyTransport(),
)

SPEC = module_spec(tasks=[
  task("remote_only", steps=[cmd("echo", "ok")], execution=RemoteOnly(REMOTE)),
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
            assert_eq!(remote.transport_kind, RemoteTransportKind::Any);
        }
        other => panic!("expected requirements-based remote execution, got {other:?}"),
    }
}
