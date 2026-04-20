#![allow(clippy::await_holding_lock)]

use std::env;
use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec, TaskLabel};
use tak_exec::{RemoteWorkerExecutionSpec, execute_remote_worker_steps};

use crate::support;

use support::{EnvGuard, env_lock, install_fake_docker, shell_step};

#[tokio::test]
async fn remote_worker_reports_injected_container_lifecycle_failure() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    configure_fake_docker_env(temp.path(), &mut env_guard);
    env_guard.set("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES", "builder-a:runtime");

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = RemoteWorkerExecutionSpec {
        task_label: TaskLabel {
            package: "//".to_string(),
            name: "remote_runtime_failure".to_string(),
        },
        attempt: 1,
        steps: vec![shell_step("true")],
        timeout_s: None,
        runtime: Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        node_id: "builder-a".to_string(),
    };

    let err = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect_err("runtime failure should surface");

    assert!(
        err.to_string()
            .contains("container lifecycle runtime failed")
    );
}

fn configure_fake_docker_env(root: &std::path::Path, env_guard: &mut EnvGuard) {
    let bin_root = root.join("bin");
    install_fake_docker(&bin_root);
    env_guard.set(
        "PATH",
        format!(
            "{}:{}",
            bin_root.display(),
            env::var("PATH").unwrap_or_default()
        ),
    );
    env_guard.set("TAK_TEST_HOST_PLATFORM", "other");
}
