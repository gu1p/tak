use std::fs;

use tak_core::model::{
    ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RemoteTransportKind, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

#[test]
fn loads_namespaced_local_remote_runtime_and_session_surface() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("docker")).expect("docker dir");
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n").expect("dockerfile");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"IMAGE_RUNTIME = Runtime.Image("alpine:3.20")
DOCKER_RUNTIME = Runtime.Dockerfile(path("docker/Dockerfile"))
SESSION = session(
  "container-check",
  execution=Execution.Local(runtime=DOCKER_RUNTIME),
  reuse=SessionReuse.Workspace(),
)

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("host", steps=[cmd("true")], execution=Execution.Local()),
    task("explicit_host", steps=[cmd("true")], execution=Execution.Local(runtime=Runtime.Host())),
    task("local_image", steps=[cmd("true")], execution=Execution.Local(runtime=IMAGE_RUNTIME)),
    task("remote_image", steps=[cmd("true")], execution=Execution.Remote(
      pool="build",
      transport=Transport.DirectHttps(),
      runtime=IMAGE_RUNTIME,
    )),
    task("session_user", steps=[cmd("true")], execution=Execution.Session("container-check")),
  ],
)
SPEC
"#,
    )
    .expect("write TASKS.py");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");

    assert_local_runtime(&spec, "host", None);
    assert_local_runtime(&spec, "explicit_host", None);
    assert_local_runtime(&spec, "local_image", Some("alpine:3.20"));
    let remote = task_execution(&spec, "remote_image");
    match remote {
        TaskExecutionSpec::RemoteOnly(remote) => {
            assert_eq!(remote.pool.as_deref(), Some("build"));
            assert_eq!(remote.transport_kind, RemoteTransportKind::Direct);
            assert_runtime_image(remote.runtime.as_ref(), "alpine:3.20");
        }
        other => panic!("expected remote execution, got {other:?}"),
    }
    assert!(spec.sessions.contains_key("container-check"));
}

fn assert_local_runtime(
    spec: &tak_core::model::WorkspaceSpec,
    task_name: &str,
    expected_image: Option<&str>,
) {
    match task_execution(spec, task_name) {
        TaskExecutionSpec::LocalOnly(local) => match expected_image {
            Some(image) => assert_runtime_image(local.runtime.as_ref(), image),
            None => assert!(local.runtime.is_none(), "expected host runtime"),
        },
        other => panic!("expected local execution for {task_name}, got {other:?}"),
    }
}

fn assert_runtime_image(runtime: Option<&RemoteRuntimeSpec>, expected: &str) {
    match runtime.expect("runtime") {
        RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
        } => assert_eq!(image, expected),
        other => panic!("expected image runtime, got {other:?}"),
    }
}

fn task_execution<'a>(
    spec: &'a tak_core::model::WorkspaceSpec,
    task_name: &str,
) -> &'a TaskExecutionSpec {
    &spec
        .tasks
        .values()
        .find(|task| task.label.name == task_name)
        .expect("task")
        .execution
}
