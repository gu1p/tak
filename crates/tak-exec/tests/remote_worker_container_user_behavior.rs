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
async fn remote_worker_container_runtime_applies_cpu_quota_and_parallelism_caps() {
    let create = run_container_task(
        None,
        Some(ContainerResourceLimitsSpec {
            cpu_cores: Some(1.5),
            memory_mb: Some(768),
        }),
    )
    .await;

    // CPU reservation is enforced as a real cgroup quota: 1.5 cores -> 1.5e9 nano-CPUs.
    // CPU quota only throttles; it never kills a container.
    assert_eq!(create.nano_cpus, Some(1_500_000_000));
    // Memory is intentionally NEVER hard-capped: a cgroup memory limit would let the
    // kernel OOM-kill the container for over-using memory, which Tak must never do.
    // Memory pressure is handled by throttling + admission, not by killing. None of the
    // memory-kill knobs may ever be set.
    assert_eq!(create.memory, None);
    assert_eq!(create.memory_swap, None);
    assert_eq!(create.oom_kill_disable, None);
    // Parallelism is capped to the floored core count so the test/doctest harness
    // and rayon cannot fan out to the host core count. floor(1.5) == 1.
    assert!(
        create.env.contains(&"RUST_TEST_THREADS=1".to_string()),
        "expected RUST_TEST_THREADS default derived from cpu_cores: {:?}",
        create.env
    );
    assert!(
        create.env.contains(&"RAYON_NUM_THREADS=1".to_string()),
        "expected RAYON_NUM_THREADS default derived from cpu_cores: {:?}",
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
