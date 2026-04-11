//! Contract tests for bollard-backed container runtime integration.

#[test]
fn container_runtime_embeds_bollard_lifecycle_calls() {
    let source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        include_str!("../src/lib.rs"),
        include_str!("../src/container_runtime.rs"),
        include_str!("../src/container_runtime/foundation.rs"),
        include_str!("../src/container_runtime/build_context.rs"),
        include_str!("../src/container_runtime/tar_archive.rs"),
        include_str!("../src/container_runtime/execution.rs"),
        include_str!("../src/container_runtime/log_stream.rs")
    );
    assert!(
        source.contains("bollard::Docker"),
        "container runtime must use bollard Docker client for lifecycle control"
    );
    assert!(
        source.contains("create_container("),
        "container runtime must create containers through bollard APIs"
    );
    assert!(
        source.contains("inspect_image("),
        "container runtime must inspect local image availability before pulling"
    );
    assert!(
        source.contains("build_image("),
        "container runtime must build Dockerfile-backed images through bollard APIs"
    );
    assert!(
        source.contains("start_container("),
        "container runtime must start containers through bollard APIs"
    );
    assert!(
        source.contains("LogsOptions"),
        "container runtime must configure bollard log streaming options"
    );
    assert!(
        source.contains("LogOutput"),
        "container runtime must map bollard log frames to tak output streams"
    );
    assert!(
        source.contains("logs::<"),
        "container runtime must stream container logs while the task is running"
    );
}
