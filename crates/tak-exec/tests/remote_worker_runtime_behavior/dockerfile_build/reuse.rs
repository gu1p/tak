use std::fs;
use std::time::Duration;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_reuses_same_dockerfile_runtime_image_for_unchanged_context() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("docker")).expect("create docker dir");
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\nRUN printf 'built\\n' > /tmp/built.txt\n",
    )
    .expect("write dockerfile");
    let spec = worker_spec(
        "remote_runtime_dockerfile_reuse",
        vec![shell_step("true")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Dockerfile {
                dockerfile: PathRef {
                    anchor: PathAnchor::Workspace,
                    path: "docker/Dockerfile".to_string(),
                },
                build_context: PathRef {
                    anchor: PathAnchor::Workspace,
                    path: ".".to_string(),
                },
            },
        }),
        "builder-a",
    );

    let first = tokio::spawn({
        let workspace_root = workspace_root.clone();
        let spec = spec.clone();
        async move { execute_remote_worker_steps(&workspace_root, &spec).await }
    });
    wait_for_build_count(&daemon, 1).await;
    daemon.release_container_exit();
    let first = first
        .await
        .expect("join first worker")
        .expect("first dockerfile runtime execution should succeed");
    assert!(first.success);

    let second = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect("second dockerfile runtime execution should succeed");
    assert!(second.success);

    let builds = daemon.build_records();
    assert_eq!(
        builds.len(),
        1,
        "unchanged Dockerfile context should build once"
    );
    assert!(
        builds[0].image_tag.starts_with("tak-runtime-"),
        "runtime image should use stable tak-runtime tag: {:?}",
        builds[0]
    );
}

async fn wait_for_build_count(daemon: &FakeDockerDaemon, expected: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if daemon.build_records().len() >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("build request should reach fake docker daemon");
}
