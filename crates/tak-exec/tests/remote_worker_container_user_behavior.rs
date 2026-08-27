#![allow(clippy::await_holding_lock)]

use tak_core::model::ContainerResourceLimitsSpec;

#[path = "remote_worker_container_user_behavior/support.rs"]
mod support;

use support::run_container_task;

#[tokio::test]
async fn remote_worker_container_runtime_passes_configured_user_to_docker() {
    let create = run_container_task(Some("1000:1000"), None).await;

    assert_eq!(create.user.as_deref(), Some("1000:1000"));
}

#[tokio::test]
async fn remote_worker_container_runtime_omits_user_for_image_default() {
    let create = run_container_task(None, None).await;

    assert_eq!(create.user, None);
}

#[tokio::test]
async fn remote_worker_container_runtime_keeps_authored_resources_as_scheduling_reservations() {
    let create = run_container_task(
        None,
        Some(ContainerResourceLimitsSpec {
            cpu_cores: Some(1.5),
            memory_mb: Some(768),
        }),
    )
    .await;

    // Authored resources reserve aggregate worker capacity. They do not become
    // per-container limits or process-global thread-pool overrides.
    assert_eq!(create.nano_cpus, None);
    assert_eq!(create.memory, None);
    assert_eq!(create.memory_swap, None);
    assert_eq!(create.oom_kill_disable, None);
    assert!(
        !create
            .env
            .iter()
            .any(|value| value.starts_with("RUST_TEST_THREADS=")),
        "resource reservations must not override RUST_TEST_THREADS: {:?}",
        create.env
    );
    assert!(
        !create
            .env
            .iter()
            .any(|value| value.starts_with("RAYON_NUM_THREADS=")),
        "resource reservations must not override RAYON_NUM_THREADS: {:?}",
        create.env
    );
}

#[tokio::test]
async fn remote_worker_container_runtime_omits_limits_without_resources() {
    let create = run_container_task(None, None).await;

    assert_eq!(create.nano_cpus, None);
    assert_eq!(create.memory, None);
    assert!(
        !create
            .env
            .iter()
            .any(|var| var.starts_with("RUST_TEST_THREADS=")),
        "RUST_TEST_THREADS should not be injected without a CPU reservation: {:?}",
        create.env
    );
}
