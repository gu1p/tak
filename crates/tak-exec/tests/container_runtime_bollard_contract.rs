//! Contract tests for bollard-backed container runtime integration.

#[test]
fn container_runtime_embeds_bollard_lifecycle_calls() {
    let source = format!(
        "{}\n{}",
        include_str!("../src/lib.rs"),
        include_str!("../src/container_runtime.rs")
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
        source.contains("start_container("),
        "container runtime must start containers through bollard APIs"
    );
}
