#![allow(clippy::await_holding_lock)]

use std::env;
use std::fs;

use tak_core::model::RemoteRuntimeSpec;
use tak_exec::{RemoteWorkerExecutionSpec, execute_remote_worker_steps};

mod support;

use support::{EnvGuard, env_lock, install_fake_docker, shell_step};

#[tokio::test]
async fn remote_worker_simulated_container_runtime_sets_expected_env() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_root = temp.path().join("bin");
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

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = RemoteWorkerExecutionSpec {
        steps: vec![shell_step(
            "printf '%s,%s,%s' \"$TAK_REMOTE_RUNTIME\" \"$TAK_REMOTE_ENGINE\" \"$TAK_REMOTE_CONTAINER_IMAGE\" > runtime.txt",
        )],
        timeout_s: None,
        runtime: Some(RemoteRuntimeSpec::Containerized {
            image: "alpine:3.20".to_string(),
        }),
        node_id: "builder-a".to_string(),
    };

    let result = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect("simulated remote worker should succeed");

    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
    assert_eq!(
        fs::read_to_string(workspace_root.join("runtime.txt")).expect("runtime marker"),
        "containerized,docker,alpine:3.20"
    );
}

#[tokio::test]
async fn remote_worker_reports_injected_container_lifecycle_failure() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_root = temp.path().join("bin");
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
    env_guard.set("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES", "builder-a:runtime");

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = RemoteWorkerExecutionSpec {
        steps: vec![shell_step("true")],
        timeout_s: None,
        runtime: Some(RemoteRuntimeSpec::Containerized {
            image: "alpine:3.20".to_string(),
        }),
        node_id: "builder-a".to_string(),
    };

    let err = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect_err("runtime failure should surface");

    assert!(
        err.to_string()
            .contains("container lifecycle runtime failed"),
        "unexpected lifecycle failure: {err:#}"
    );
}
