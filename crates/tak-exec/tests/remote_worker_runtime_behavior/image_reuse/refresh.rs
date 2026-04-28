use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{ImageCacheOptions, RemoteWorkerExecutionSpec, execute_remote_worker_steps};

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

#[path = "refresh/previous_id.rs"]
mod previous_id;

#[tokio::test]
async fn remote_worker_refreshes_mutable_image_after_ttl_then_reuses_again() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    daemon.remove_image("alpine:3.20");
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let db_path = temp.path().join("agent.sqlite");
    let spec = mutable_image_spec(db_path.clone());

    daemon.release_container_exit();
    run_worker(&workspace_root, &spec).await;
    assert_eq!(daemon.pull_records().len(), 1);

    let conn = rusqlite::Connection::open(&db_path).expect("open image cache db");
    conn.execute(
        "UPDATE image_cache_entries SET last_refreshed_at_ms = 0",
        [],
    )
    .expect("age mutable image cache entry");

    run_worker(&workspace_root, &spec).await;
    assert_eq!(
        daemon.pull_records().len(),
        2,
        "expired mutable tag should be pulled again"
    );

    run_worker(&workspace_root, &spec).await;
    assert_eq!(
        daemon.pull_records().len(),
        2,
        "fresh mutable tag should be reused within its ttl"
    );
}

fn mutable_image_spec(db_path: std::path::PathBuf) -> RemoteWorkerExecutionSpec {
    mutable_image_spec_for("alpine:3.20", db_path)
}

pub(super) fn mutable_image_spec_for(
    image: &str,
    db_path: std::path::PathBuf,
) -> RemoteWorkerExecutionSpec {
    let mut spec = worker_spec(
        "remote_runtime_mutable_image_ttl",
        vec![shell_step("true")],
        None,
        Some(RemoteRuntimeSpec::Containerized {
            source: ContainerRuntimeSourceSpec::Image {
                image: image.to_string(),
            },
        }),
        "builder-a",
    );
    spec.image_cache = Some(ImageCacheOptions {
        db_path,
        budget_bytes: 50_000_000_000,
        mutable_tag_ttl_secs: 3_600,
        sweep_interval_secs: 60,
        low_disk_min_free_percent: 10.0,
        low_disk_min_free_bytes: 10_000_000_000,
    });
    spec
}

pub(super) async fn run_worker(workspace_root: &std::path::Path, spec: &RemoteWorkerExecutionSpec) {
    let result = execute_remote_worker_steps(workspace_root, spec)
        .await
        .expect("mutable image runtime execution should succeed");
    assert!(result.success);
}
