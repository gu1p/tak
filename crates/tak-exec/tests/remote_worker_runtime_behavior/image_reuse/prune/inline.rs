use tak_exec::execute_remote_worker_steps;

use crate::support::{EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock};

use super::support::{mutable_image_spec, seed_cache_entry_with_image_id};

#[tokio::test]
async fn remote_worker_records_cache_without_pruning_other_cached_images_inline() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.set_image("old:latest", "sha256:old-image", 1024);
    daemon.set_image("alpine:3.20", "sha256:new-image", 1024);
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("create workspace");
    let db_path = temp.path().join("agent.sqlite");
    seed_cache_entry_with_image_id(
        &db_path,
        "image:old:latest",
        "old:latest",
        "sha256:old-image",
        1024,
    );
    let spec = mutable_image_spec(db_path);

    daemon.release_container_exit();
    let result = execute_remote_worker_steps(&workspace_root, &spec)
        .await
        .expect("mutable image runtime execution should succeed");

    assert!(result.success);
    assert!(daemon.image_removal_attempts().is_empty());
}
