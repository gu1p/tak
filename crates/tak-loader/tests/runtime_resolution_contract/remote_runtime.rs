use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec, TaskExecutionSpec};
use tak_loader::{LoadOptions, load_workspace};

use super::support::write_root_and_app_tasks;

#[test]
fn resolves_remote_dockerfile_runtime_with_explicit_build_context() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  required_capabilities=["linux"],
  runtime=Runtime.Dockerfile(path("../infra/test.Dockerfile"), build_context=path("..")),
)
SPEC = module_spec(tasks=[task("remote_only", steps=[cmd("echo", "ok")], execution=REMOTE)])
SPEC
"#,
    );

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let task = spec
        .tasks
        .values()
        .next()
        .expect("resolved task should exist");
    match &task.execution {
        TaskExecutionSpec::RemoteOnly(remote) => {
            match remote.runtime.as_ref().expect("remote runtime") {
                RemoteRuntimeSpec::Containerized { source } => match source {
                    ContainerRuntimeSourceSpec::Dockerfile {
                        dockerfile,
                        build_context,
                    } => {
                        assert_eq!(dockerfile.path, "apps/infra/test.Dockerfile");
                        assert_eq!(build_context.path, "apps");
                    }
                    other => panic!("expected dockerfile source, got {other:?}"),
                },
            }
        }
        other => panic!("expected RemoteOnly execution, got {other:?}"),
    }
}
