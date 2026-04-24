use super::run_override_runtime::resolve_container_runtime_for_task;
use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateSpec, LocalSpec, PathAnchor, PathRef,
    RemoteRuntimeSpec, ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel,
};

fn image_runtime(image: &str) -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: image.to_string(),
        },
    }
}

fn dockerfile_runtime(path: &str) -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile: PathRef {
                anchor: PathAnchor::Workspace,
                path: path.to_string(),
            },
            build_context: PathRef {
                anchor: PathAnchor::Workspace,
                path: ".".to_string(),
            },
        },
    }
}

fn resolved_task(
    execution: TaskExecutionSpec,
    container_runtime: Option<RemoteRuntimeSpec>,
) -> ResolvedTask {
    ResolvedTask {
        label: TaskLabel {
            package: "//".to_string(),
            name: "check".to_string(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime,
        execution,
        session: None,
        tags: Vec::new(),
    }
}

#[test]
fn resolve_container_runtime_prefers_declared_task_runtime_over_workspace_default() {
    let task = resolved_task(
        TaskExecutionSpec::LocalOnly(LocalSpec {
            id: "dev".to_string(),
            max_parallel_tasks: 1,
            runtime: Some(dockerfile_runtime("docker/task.Dockerfile")),
        }),
        Some(image_runtime("alpine:3.20")),
    );

    let runtime = resolve_container_runtime_for_task(&task, None).expect("declared runtime wins");

    match runtime {
        RemoteRuntimeSpec::Containerized {
            source:
                ContainerRuntimeSourceSpec::Dockerfile {
                    dockerfile,
                    build_context,
                },
        } => {
            assert_eq!(dockerfile.path, "docker/task.Dockerfile");
            assert_eq!(build_context.path, ".");
        }
        other => panic!("expected dockerfile runtime, got {other:?}"),
    }
}

#[test]
fn resolve_container_runtime_uses_workspace_default_when_task_has_no_declared_runtime() {
    let task = resolved_task(
        TaskExecutionSpec::LocalOnly(LocalSpec::default()),
        Some(image_runtime("alpine:3.20")),
    );

    let runtime =
        resolve_container_runtime_for_task(&task, None).expect("workspace default runtime");

    match runtime {
        RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
        } => assert_eq!(image, "alpine:3.20"),
        other => panic!("expected image runtime, got {other:?}"),
    }
}
