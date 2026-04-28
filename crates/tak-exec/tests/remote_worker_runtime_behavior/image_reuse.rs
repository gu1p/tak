use std::fs;

use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_exec::{ImageCacheOptions, execute_remote_worker_steps};

use crate::support::{
    EnvGuard, FakeDockerDaemon, configure_real_docker_env, env_lock, shell_step, worker_spec,
};

mod prewarmed;
mod prune;
mod refresh;
mod status;

#[tokio::test]
async fn remote_worker_reuses_present_mutable_image_when_image_cache_is_enabled() {
    let _env_lock = env_lock();
    let mut env_guard = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path());
    configure_real_docker_env(temp.path(), daemon.socket_path(), &mut env_guard);

    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let mut spec = worker_spec(
        "remote_runtime_mutable_image_reuse",
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
        db_path: temp.path().join("agent.sqlite"),
        budget_bytes: 50_000_000_000,
        mutable_tag_ttl_secs: 86_400,
        sweep_interval_secs: 60,
        low_disk_min_free_percent: 10.0,
        low_disk_min_free_bytes: 10_000_000_000,
    });

    let worker = tokio::spawn({
        let workspace_root = workspace_root.clone();
        async move { execute_remote_worker_steps(&workspace_root, &spec).await }
    });
    daemon.release_container_exit();

    let result = worker
        .await
        .expect("join remote worker")
        .expect("mutable image runtime execution should succeed");
    assert!(result.success);
    assert!(
        daemon.pull_records().is_empty(),
        "present mutable image should be reused without pulling"
    );
}
