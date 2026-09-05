use super::*;
use crate::cli::run_override_runtime::explicit_container_runtime_override;

#[test]
fn explicit_cli_container_sources_do_not_add_resource_limits() {
    for runtime in [
        explicit_container_runtime_override(Some("alpine:3.20"), None, None),
        explicit_container_runtime_override(None, Some("docker/Dockerfile"), Some("docker")),
    ] {
        let runtime = runtime.expect("valid source").expect("container runtime");
        match runtime {
            RemoteRuntimeSpec::Containerized {
                resource_limits, ..
            } => assert_eq!(resource_limits, None),
            other => panic!("expected CLI container runtime, got {other:?}"),
        }
    }
}

#[test]
fn resolve_container_runtime_uses_workspace_default_when_task_has_no_declared_runtime() {
    let task = resolved_task(
        TaskExecutionSpec::LocalOnly(LocalSpec::default()),
        Some(image_runtime("alpine:3.20")),
    );

    let runtime = resolved_runtime_for_execution_override(
        &task,
        &task.execution,
        RunPlacementSelector::Local,
        true,
        None,
    )
    .expect("default runtime")
    .expect("container runtime");

    match runtime {
        RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
            ..
        } => assert_eq!(image, "alpine:3.20"),
        other => panic!("expected image runtime, got {other:?}"),
    }
}
