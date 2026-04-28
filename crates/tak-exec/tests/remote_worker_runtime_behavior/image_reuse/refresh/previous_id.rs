use std::fs;

use crate::support::{EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock};

use super::{mutable_image_spec_for, run_worker};

#[tokio::test]
async fn mutable_refresh_keeps_previous_image_id_tracked_for_later_pruning() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.set_image("alpine:latest", "sha256:old-image", 1024);
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let db_path = temp.path().join("agent.sqlite");
    let spec = mutable_image_spec_for("alpine:latest", db_path.clone());

    daemon.release_container_exit();
    run_worker(&workspace_root, &spec).await;
    let initial =
        tak_exec::image_cache_status(&db_path, 50_000, 0.0, 0).expect("initial cache status");
    assert_eq!(initial.entry_count, 1);
    assert_eq!(initial.used_bytes, 1024);

    let conn = rusqlite::Connection::open(&db_path).expect("open image cache db");
    conn.execute(
        "UPDATE image_cache_entries SET last_refreshed_at_ms = 0",
        [],
    )
    .expect("age mutable image cache entry");
    daemon.set_image("alpine:latest", "sha256:new-image", 2048);

    run_worker(&workspace_root, &spec).await;
    let refreshed =
        tak_exec::image_cache_status(&db_path, 50_000, 0.0, 0).expect("refreshed cache status");
    assert_eq!(refreshed.entry_count, 2);
    assert_eq!(refreshed.used_bytes, 3072);
}
