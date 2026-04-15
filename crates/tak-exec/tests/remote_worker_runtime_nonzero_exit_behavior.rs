#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

#[path = "support/nonzero_wait_docker_daemon.rs"]
mod nonzero_wait_docker_daemon;
mod support;

use nonzero_wait_docker_daemon::NonzeroWaitDockerDaemon;
use support::{EnvGuard, configure_real_docker_env, env_lock, shell_step, worker_spec};

#[tokio::test]
async fn remote_worker_reports_nonzero_docker_wait_as_task_failure() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = NonzeroWaitDockerDaemon::spawn(
        temp.path(),
        1,
        "line-limit-check: crates/takd/tests/support/fake_docker_daemon/handlers.rs:102 exceeds limit 100\n",
    );
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = worker_spec(
        "remote_runtime_nonzero_exit",
        vec![shell_step("printf 'lint failed\\n' >&2; exit 1")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        "builder-a",
    );

    let result = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect("nonzero container exit should return a task result");

    assert!(!result.success);
    assert_eq!(result.exit_code, Some(1));
    assert_eq!(result.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
}

#[tokio::test]
async fn remote_worker_preserves_docker_wait_error_as_infra_failure() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = NonzeroWaitDockerDaemon::spawn_with_wait_error(
        temp.path(),
        1,
        Some("context canceled"),
        "docker runtime canceled\n",
    );
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = worker_spec(
        "remote_runtime_wait_error",
        vec![shell_step("printf 'lint failed\\n' >&2; exit 1")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        "builder-a",
    );

    let err = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect_err("docker wait errors should surface as infra failures");

    assert!(
        err.to_string()
            .contains("container lifecycle runtime failed")
    );
    assert!(err.to_string().contains("context canceled"));
}
