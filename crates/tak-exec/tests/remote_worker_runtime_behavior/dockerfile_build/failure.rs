use std::fs;
use std::sync::Arc;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::{OutputStream, TaskOutputObserver, execute_remote_worker_steps_with_output};

use crate::support::{
    CollectingObserver, EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock,
    shell_step, worker_spec,
};

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
            resource_limits: None,
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
    let stdout = stream_text(&chunks, OutputStream::Stdout);
    let stderr = stream_text(&chunks, OutputStream::Stderr);
    assert!(
        stdout.contains("Step 1/1 : RUN failing build step"),
        "stdout chunks:\n{stdout}"
    );
    assert!(
        stderr.contains("failed to solve: process returned a non-zero code"),
        "stderr chunks:\n{stderr}"
    );
}

fn stream_text(chunks: &[tak_exec::TaskOutputChunk], stream: OutputStream) -> String {
    let bytes = chunks
        .iter()
        .filter(|chunk| chunk.stream == stream)
        .flat_map(|chunk| chunk.bytes.clone())
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).into_owned()
}
