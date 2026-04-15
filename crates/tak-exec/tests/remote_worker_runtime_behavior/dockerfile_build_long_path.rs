use std::fs;
use std::time::Duration;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_build_context_supports_long_relative_paths() {
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

    let long_relative = format!(
        "{}/{}/{}/{}/artifact.txt",
        "segment1234567890segment1234567890segment1234567890segment1234567890",
        "segment2234567890segment2234567890segment2234567890segment2234567890",
        "segment3234567890segment3234567890segment3234567890segment3234567890",
        "segment4234567890segment4234567890segment4234567890segment4234567890",
    );
    assert!(
        long_relative.len() > 255,
        "test path should exceed tar ustar limit"
    );
    let long_path = workspace_root.join(&long_relative);
    fs::create_dir_all(long_path.parent().expect("long path parent")).expect("long path dirs");
    fs::write(&long_path, "long path payload\n").expect("write long path file");

    let spec = worker_spec(
        "remote_runtime_dockerfile_long_path",
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
    let build = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(build) = daemon.single_build() {
                break build;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("build request should reach fake docker daemon");
    daemon.release_container_exit();

    let result = worker
        .await
        .expect("join remote worker")
        .expect("dockerfile runtime execution should succeed");
    assert!(result.success);
    assert!(
        build
            .context_entries
            .iter()
            .any(|entry| entry == &long_relative),
        "expected long path entry in build context: {:?}",
        build.context_entries
    );
}
