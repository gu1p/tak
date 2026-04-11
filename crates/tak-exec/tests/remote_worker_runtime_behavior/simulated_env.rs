use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support::{EnvGuard, configure_fake_docker_env, env_lock, shell_step, worker_spec};

#[tokio::test]
async fn remote_worker_simulated_container_runtime_sets_expected_env() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    configure_fake_docker_env(temp.path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let spec = worker_spec(
        "remote_runtime_env",
        vec![shell_step(
            "printf '%s,%s,%s' \"$TAK_REMOTE_RUNTIME\" \"$TAK_REMOTE_ENGINE\" \"$TAK_REMOTE_CONTAINER_IMAGE\" > runtime.txt",
        )],
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
