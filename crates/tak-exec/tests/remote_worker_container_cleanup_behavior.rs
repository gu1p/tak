#![allow(clippy::await_holding_lock)]

use std::{fs, sync::Arc};

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{execute_remote_worker_steps, execute_remote_worker_steps_with_output};

use crate::support::{
    CollectingObserver, EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock,
    shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_removes_container_after_start_failure() {
    let (temp, daemon, workspace_root, spec, _env_lock, mut env) = cleanup_case();
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env);
    daemon.fail_start("start refused");

    let err = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect_err("start failure should surface");

    assert!(err.to_string().contains("start container failed"));
    assert_eq!(daemon.create_records().len(), 1);
    assert_eq!(removed_container_ids(&daemon), vec!["container-123"]);
}

#[tokio::test]
async fn remote_worker_removes_container_when_log_stream_fails() {
    let (temp, daemon, workspace_root, spec, _env_lock, mut env) = cleanup_case();
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env);
    daemon.fail_logs("logs unavailable");
    daemon.release_container_exit();

    let observer = Arc::new(CollectingObserver::default());
    let _err = execute_remote_worker_steps_with_output(&workspace_root, &spec, Some(observer))
        .await
        .expect_err("log failure should surface");

    assert_eq!(daemon.create_records().len(), 1);
    assert_eq!(removed_container_ids(&daemon), vec!["container-123"]);
}

fn cleanup_case() -> (
    tempfile::TempDir,
    FakeDockerDaemon,
    std::path::PathBuf,
    tak_exec::RemoteWorkerExecutionSpec,
    impl Drop,
    EnvGuard,
) {
    let env_lock = env_lock();
    let env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = worker_spec(
        "cleanup",
        vec![shell_step("true")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
            resource_limits: None,
        }),
        "builder-a",
    );
    (temp, daemon, workspace_root, spec, env_lock, env)
}

fn removed_container_ids(daemon: &FakeDockerDaemon) -> Vec<String> {
    daemon
        .remove_records()
        .into_iter()
        .map(|record| record.container_id)
        .collect()
}
