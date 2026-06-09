#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support;

use support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_container_runtime_passes_configured_user_to_docker() {
    let create = run_container_task_with_user(Some("1000:1000")).await;

    assert_eq!(create.user.as_deref(), Some("1000:1000"));
}

#[tokio::test]
async fn remote_worker_container_runtime_omits_user_for_image_default() {
    let create = run_container_task_with_user(None).await;

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

async fn run_container_task_with_user(
    user: Option<&str>,
) -> support::fake_docker_daemon::CreateRecord {
    run_container_task(user, None).await
}

async fn run_container_task(
    user: Option<&str>,
    resource_limits: Option<ContainerResourceLimitsSpec>,
) -> support::fake_docker_daemon::CreateRecord {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let mut spec = worker_spec(
        "remote_runtime_container_user",
        vec![shell_step("printf 'containerized execution'")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
            resource_limits,
        }),
        "builder-a",
    );
    spec.container_user = user.map(ToString::to_string);

    let worker = tokio::spawn({
        let workspace_root = workspace_root.clone();
        async move { execute_remote_worker_steps(&workspace_root, &spec).await }
    });
    daemon.release_container_exit();

    let result = worker
        .await
        .expect("join remote worker")
        .expect("container runtime execution should succeed");
    assert!(result.success);

    let creates = daemon.create_records();
    assert_eq!(creates.len(), 1);
    assert!(
        creates[0]
            .binds
            .iter()
            .any(|bind| bind.starts_with(&workspace_root.display().to_string())),
        "workspace should still be bind-mounted: {creates:?}"
    );
    creates[0].clone()
}
