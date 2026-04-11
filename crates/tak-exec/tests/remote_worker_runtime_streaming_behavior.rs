#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{OutputStream, TaskOutputChunk, execute_remote_worker_steps_with_output};
use tokio::time::{Duration, timeout};

mod support;

use support::{
    CollectingObserver, EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock,
    shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_container_runtime_streams_logs_to_output_observer_while_running() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = worker_spec(
        "remote_runtime_streams_logs",
        vec![shell_step("printf 'containerized execution'")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        "builder-a",
    );
    let observer = Arc::new(CollectingObserver::default());
    let mut worker = tokio::spawn({
        let workspace_root = workspace_root.clone();
        let spec = spec.clone();
        let observer = observer.clone();
        async move {
            execute_remote_worker_steps_with_output(&workspace_root, &spec, Some(observer)).await
        }
    });

    timeout(Duration::from_secs(3), observer.wait_for_chunks(1))
        .await
        .expect("container logs should reach observer while task is running");
    assert_eq!(
        observer.snapshot().clone(),
        vec![TaskOutputChunk {
            task_label: spec.task_label.clone(),
            attempt: 1,
            stream: OutputStream::Stdout,
            bytes: b"hello from container\n".to_vec(),
        }]
    );
    assert!(
        timeout(Duration::from_millis(100), &mut worker)
            .await
            .is_err()
    );

    daemon.release_container_exit();

    let result = timeout(Duration::from_secs(3), &mut worker)
        .await
        .expect("worker should finish once fake docker releases exit")
        .expect("join remote worker")
        .expect("container runtime execution should succeed");
    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
}
