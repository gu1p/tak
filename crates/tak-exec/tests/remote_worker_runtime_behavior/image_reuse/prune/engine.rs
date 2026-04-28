use tak_exec::run_image_cache_janitor_once;

use crate::support::{EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock};

use super::support::{cache_options, seed_cache_entry_for_engine};

#[tokio::test]
async fn image_cache_janitor_uses_recorded_podman_engine() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let docker_root = temp.path().join("docker");
    std::fs::create_dir_all(&docker_root).expect("create docker root");
    let docker_daemon = FakeDockerDaemon::spawn(&docker_root);
    let podman_root = temp.path().join("podman");
    std::fs::create_dir_all(&podman_root).expect("create podman root");
    let podman_daemon = FakeDockerDaemon::spawn(&podman_root);
    podman_daemon.set_image("podman:latest", "sha256:podman-image", 1024);
    configure_real_docker_env(temp.path(), docker_daemon.socket_path(), &mut env_guard);
    env_guard.set(
        "TAK_PODMAN_SOCKET",
        format!("unix://{}", podman_daemon.socket_path().display()),
    );

    let options = cache_options(temp.path().join("agent.sqlite"));
    seed_cache_entry_for_engine(
        &options.db_path,
        "podman",
        "image:podman:latest",
        "podman:latest",
        "sha256:podman-image",
        1024,
    );
    run_image_cache_janitor_once(&options)
        .await
        .expect("image cache janitor should prune podman rows through podman");

    assert!(docker_daemon.image_removal_attempts().is_empty());
    assert_eq!(
        podman_daemon.image_removal_attempts(),
        vec!["sha256:podman-image"]
    );
}

#[tokio::test]
async fn image_cache_janitor_skips_unavailable_recorded_engine_and_prunes_later_engine() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let podman_root = temp.path().join("podman");
    std::fs::create_dir_all(&podman_root).expect("create podman root");
    let podman_daemon = FakeDockerDaemon::spawn(&podman_root);
    podman_daemon.set_image("podman:latest", "sha256:podman-image", 1024);
    configure_real_docker_env(
        temp.path(),
        &temp.path().join("missing-docker.sock"),
        &mut env_guard,
    );
    env_guard.set(
        "TAK_PODMAN_SOCKET",
        format!("unix://{}", podman_daemon.socket_path().display()),
    );

    let options = cache_options(temp.path().join("agent.sqlite"));
    seed_cache_entry_for_engine(
        &options.db_path,
        "docker",
        "image:docker:latest",
        "docker:latest",
        "sha256:docker-image",
        1024,
    );
    seed_cache_entry_for_engine(
        &options.db_path,
        "podman",
        "image:podman:latest",
        "podman:latest",
        "sha256:podman-image",
        1024,
    );
    run_image_cache_janitor_once(&options)
        .await
        .expect("unavailable stale engines should not block later recorded engines");

    assert_eq!(
        podman_daemon.image_removal_attempts(),
        vec!["sha256:podman-image"]
    );
}
