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
async fn remote_worker_container_runtime_passes_resource_limits_to_docker() {
    let create = run_container_task(
        None,
        Some(ContainerResourceLimitsSpec {
            cpu_cores: Some(1.5),
            memory_mb: Some(768),
        }),
    )
    .await;

    assert_eq!(create.nano_cpus, Some(1_500_000_000));
    assert_eq!(create.memory, Some(805_306_368));
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
