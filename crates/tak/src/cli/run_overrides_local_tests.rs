use super::run_overrides::{RunExecutionOverrideArgs, apply_run_execution_overrides};
use super::run_overrides_test_support::{
    image_runtime, resolved_task, task_label, workspace_with_task,
};
use tak_core::model::{
    ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RemoteSpec, RemoteTransportKind,
    TaskExecutionSpec,
};

#[test]
fn local_override_preserves_existing_runtime_without_container_flag() {
    let label = task_label("check");
    let spec = workspace_with_task(resolved_task(
        label.clone(),
        TaskExecutionSpec::RemoteOnly(RemoteSpec {
            pool: Some("build".to_string()),
            required_tags: vec!["builder".to_string()],
            required_capabilities: vec!["linux".to_string()],
            transport_kind: RemoteTransportKind::Direct,
            runtime: Some(image_runtime("alpine:3.20")),
        }),
    ));

    let overridden = apply_run_execution_overrides(
        &spec,
        std::slice::from_ref(&label),
        RunExecutionOverrideArgs {
            local: true,
            remote: false,
            container: false,
            container_image: None,
            container_dockerfile: None,
            container_build_context: None,
        },
    )
    .expect("apply local override");

    match &overridden.tasks.get(&label).expect("task").execution {
        TaskExecutionSpec::LocalOnly(local) => match local.runtime.as_ref().expect("runtime") {
            RemoteRuntimeSpec::Containerized {
                source: ContainerRuntimeSourceSpec::Image { image },
            } => assert_eq!(image, "alpine:3.20"),
            other => panic!("expected image runtime, got {other:?}"),
        },
        other => panic!("expected local execution, got {other:?}"),
    }
}
