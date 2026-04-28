use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{ImageCacheOptions, RemoteWorkerExecutionSpec, execute_remote_worker_steps};

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[tokio::test]
async fn prewarmed_mutable_image_is_recorded_and_later_refreshed_after_ttl() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let db_path = temp.path().join("agent.sqlite");
    let spec = mutable_image_spec(db_path.clone());

    daemon.release_container_exit();
    run_worker(&workspace_root, &spec).await;
    assert!(daemon.pull_records().is_empty());

    let status = tak_exec::image_cache_status(&db_path, 50_000, 0.0, 0).expect("cache status");
    assert_eq!(status.entry_count, 1);
    assert_eq!(status.used_bytes, 1024);

    let conn = rusqlite::Connection::open(&db_path).expect("open image cache db");
    conn.execute(
        "UPDATE image_cache_entries SET last_refreshed_at_ms = 0",
        [],
    )
    .expect("age mutable cache entry");

    run_worker(&workspace_root, &spec).await;
    assert_eq!(
        daemon.pull_records().len(),
        1,
        "expired prewarmed mutable image should be refreshed"
    );
}

fn mutable_image_spec(db_path: std::path::PathBuf) -> RemoteWorkerExecutionSpec {
    let mut spec = worker_spec(
        "remote_runtime_prewarmed_mutable_image_cache",
        vec![shell_step("true")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: "alpine:3.20".to_string(),
            },
        }),
        "builder-a",
    );
    spec.image_cache = Some(ImageCacheOptions {
        db_path,
        budget_bytes: 50_000_000_000,
        mutable_tag_ttl_secs: 1,
        sweep_interval_secs: 60,
        low_disk_min_free_percent: 0.0,
        low_disk_min_free_bytes: 0,
    });
    spec
}

async fn run_worker(workspace_root: &std::path::Path, spec: &RemoteWorkerExecutionSpec) {
    let result = execute_remote_worker_steps(workspace_root, spec)
        .await
        .expect("mutable image runtime execution should succeed");
    assert!(result.success);
}
