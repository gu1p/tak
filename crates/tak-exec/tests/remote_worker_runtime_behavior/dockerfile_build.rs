use std::fs;
use std::sync::Arc;
use std::time::Duration;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::{
    OutputStream, TaskOutputObserver, execute_remote_worker_steps,
    execute_remote_worker_steps_with_output,
};

use crate::support::{
    CollectingObserver, EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock,
    shell_step, worker_spec,
};

#[path = "dockerfile_build/reuse.rs"]
mod reuse;

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
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
    assert_eq!(build.dockerfile, "docker/Dockerfile");
    assert_eq!(
        build.context_entries,
        vec!["docker/Dockerfile", "src/input.txt"]
    );
}

#[tokio::test]
async fn remote_worker_streams_dockerfile_build_logs_and_preserves_error_detail() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.fail_build("failed to solve: process returned a non-zero code");
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("docker")).expect("create docker dir");
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\nRUN false\n",
    )
    .expect("write dockerfile");
    let spec = worker_spec(
        "remote_runtime_dockerfile_failure",
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
    let observer = Arc::new(CollectingObserver::default());
    let output_observer: Arc<dyn TaskOutputObserver> = observer.clone();

    let error =
        execute_remote_worker_steps_with_output(&workspace_root, &spec, Some(output_observer))
            .await
            .expect_err("dockerfile build failure should fail remote worker execution");
    let error_message = format!("{error:#}");

    assert!(
        error_message.contains("infra error: container lifecycle build failed: failed to solve"),
        "error:\n{error_message}"
    );

    let chunks = observer.snapshot();
    let stdout = chunks
        .iter()
        .filter(|chunk| chunk.stream == OutputStream::Stdout)
        .flat_map(|chunk| chunk.bytes.clone())
        .collect::<Vec<_>>();
    let stderr = chunks
        .iter()
        .filter(|chunk| chunk.stream == OutputStream::Stderr)
        .flat_map(|chunk| chunk.bytes.clone())
        .collect::<Vec<_>>();
    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);

    assert!(
        stdout.contains("Step 1/1 : RUN failing build step"),
        "stdout chunks:\n{stdout}"
    );
    assert!(
        stderr.contains("failed to solve: process returned a non-zero code"),
        "stderr chunks:\n{stderr}"
    );
}
