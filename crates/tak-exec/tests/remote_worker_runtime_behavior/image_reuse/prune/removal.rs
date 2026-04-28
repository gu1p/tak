use tak_exec::{image_cache_status, run_image_cache_janitor_once};

use crate::support::{EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock};

use super::support::{cache_options, seed_cache_entry, seed_cache_entry_with_image_id};

#[tokio::test]
async fn image_cache_prune_keeps_row_when_image_removal_fails() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.fail_image_removal(409);
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let options = cache_options(temp.path().join("agent.sqlite"));
    seed_cache_entry(&options.db_path, "image:alpine:3.20", "alpine:3.20");
    run_image_cache_janitor_once(&options)
        .await
        .expect("image cache janitor should tolerate failed removal");

    let status =
        image_cache_status(&options.db_path, options.budget_bytes, 0.0, 0).expect("cache status");
    assert_eq!(status.entry_count, 1);
    assert_eq!(status.used_bytes, 1024);
    assert_eq!(daemon.image_removal_attempts(), vec!["sha256:test-image"]);
}

#[tokio::test]
async fn image_cache_prune_deletes_row_when_image_is_already_missing() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let options = cache_options(temp.path().join("agent.sqlite"));
    seed_cache_entry(&options.db_path, "image:missing", "missing:latest");
    run_image_cache_janitor_once(&options)
        .await
        .expect("image cache janitor should drop missing image row");

    let status =
        image_cache_status(&options.db_path, options.budget_bytes, 0.0, 0).expect("cache status");
    assert_eq!(status.entry_count, 0);
    assert_eq!(status.used_bytes, 0);
    assert_eq!(daemon.image_removal_attempts(), vec!["sha256:test-image"]);
}

#[tokio::test]
async fn image_cache_prune_removes_recorded_image_id_not_moved_mutable_tag() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.set_image("alpine:latest", "sha256:new-image", 2048);
    daemon.set_image("old-ref", "sha256:old-image", 1024);
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let options = cache_options(temp.path().join("agent.sqlite"));
    seed_cache_entry_with_image_id(
        &options.db_path,
        "image:alpine:latest",
        "alpine:latest",
        "sha256:old-image",
        1024,
    );
    run_image_cache_janitor_once(&options)
        .await
        .expect("image cache janitor should prune by recorded image id");

    assert_eq!(daemon.image_removal_attempts(), vec!["sha256:old-image"]);
}
