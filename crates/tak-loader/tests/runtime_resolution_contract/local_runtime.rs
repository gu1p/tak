use tak_core::model::{
    ContainerRuntimeSourceSpec, PathAnchor, RemoteRuntimeSpec, TaskExecutionSpec,
};
use tak_loader::{LoadOptions, load_workspace};

use super::support::write_root_and_app_tasks;

#[test]
fn resolves_local_dockerfile_runtime_with_default_build_context_to_package_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_root_and_app_tasks(
        temp.path(),
        r#"
LOCAL = Local(id="dev", runtime=DockerfileRuntime(dockerfile=path("docker/Dockerfile")))
SPEC = module_spec(tasks=[task("local_only", steps=[cmd("echo", "ok")], execution=LocalOnly(LOCAL))])
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
        TaskExecutionSpec::LocalOnly(local) => match local.runtime.as_ref().expect("local runtime")
        {
            RemoteRuntimeSpec::Containerized { source } => match source {
                ContainerRuntimeSourceSpec::Dockerfile {
                    dockerfile,
                    build_context,
                } => {
                    assert_eq!(dockerfile.anchor, PathAnchor::Workspace);
                    assert_eq!(dockerfile.path, "apps/web/docker/Dockerfile");
                    assert_eq!(build_context.anchor, PathAnchor::Workspace);
                    assert_eq!(build_context.path, "apps/web");
                }
                other => panic!("expected dockerfile source, got {other:?}"),
            },
        },
        other => panic!("expected LocalOnly execution, got {other:?}"),
    }
}
