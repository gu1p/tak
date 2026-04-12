use std::fs;
use std::os::unix::fs::PermissionsExt;

use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, RemoteRuntimeSpec};
use tak_exec::execute_remote_worker_steps;

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn remote_worker_preserves_executable_bits_in_dockerfile_build_context() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(workspace_root.join("docker")).expect("create docker dir");
    fs::create_dir_all(workspace_root.join("scripts")).expect("create scripts dir");
    fs::write(
        workspace_root.join("docker/Dockerfile"),
        "FROM alpine:3.20\n\
         COPY scripts/setup.sh /usr/local/bin/setup.sh\n\
         RUN /usr/local/bin/setup.sh\n",
    )
    .expect("write dockerfile");
    let script = workspace_root.join("scripts/setup.sh");
    fs::write(&script, "#!/bin/sh\necho setup\n").expect("write script");
    let mut permissions = fs::metadata(&script)
        .expect("script metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).expect("chmod script");
    let spec = worker_spec(
        "remote_runtime_exec_bits",
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

    worker
        .await
        .expect("join remote worker")
        .expect("dockerfile runtime execution should succeed");

    assert_eq!(build.context_modes.get("scripts/setup.sh"), Some(&0o755));
}
