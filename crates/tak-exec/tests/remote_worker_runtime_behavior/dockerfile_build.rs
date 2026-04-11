use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_builds_dockerfile_runtime_before_running_steps() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("docker")).expect("create docker dir");
    fs::create_dir_all(workspace_root.join("src")).expect("create src dir");
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\nCOPY src/input.txt /tmp/input.txt\n",
    )
    .expect("write dockerfile");
    fs::write(workspace_root.join("src/input.txt"), "hello\n").expect("write input");
    let spec = worker_spec(
        "remote_runtime_dockerfile",
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

    let worker = tokio::spawn({
        let workspace_root = workspace_root.clone();
        let spec = spec.clone();
        async move { execute_remote_worker_steps(&workspace_root, &spec).await }
    });

    let build = loop {
        if let Some(build) = daemon.single_build() {
            break build;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };
    daemon.release_container_exit();

    let result = worker
        .await
        .expect("join remote worker")
        .expect("dockerfile runtime execution should succeed");

    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
    assert_eq!(build.dockerfile, "docker/Dockerfile");
    assert_eq!(
        build.context_entries,
        vec!["docker/Dockerfile", "src/input.txt"]
    );
}
