use super::*;

#[test]
fn resolve_container_runtime_uses_workspace_default_when_task_has_no_declared_runtime() {
    let task = resolved_task(
        TaskExecutionSpec::LocalOnly(LocalSpec::default()),
        Some(image_runtime("alpine:3.20")),
    );

    let runtime = resolve_container_runtime_for_task(&task, None).expect("default runtime");

    match runtime {
        RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image { image },
            ..
        } => assert_eq!(image, "alpine:3.20"),
        other => panic!("expected image runtime, got {other:?}"),
    }
}
