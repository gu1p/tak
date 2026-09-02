use tak_core::v2::{ContainerSource, Step, TaskRuntime};
use tak_proto::worker_v2::payload_digest;

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::configure_fake_docker_env,
    v2_worker_capacity::{dispatch, snapshot, wait_terminal},
    v2_worker_execution::output_dispatch,
    worker_http::start_server_with_runtime_and_image_cache,
};

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn configured_worker_v2_execution_records_and_advertises_cached_images() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let exec_root = temp.path().join("exec");
    let docker = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), docker.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .build();
    let db_path = temp.path().join("image-cache.sqlite");
    let server = start_server_with_runtime_and_image_cache(
        runtime,
        takd::RemoteImageCacheRuntimeConfig {
            db_path: db_path.clone(),
            budget_bytes: 1024 * 1024,
            mutable_tag_ttl_secs: 3_600,
            sweep_interval_secs: 60,
            low_disk_min_free_percent: 0.0,
            low_disk_min_free_bytes: 0,
        },
    )
    .await;
    let mut request = output_dispatch();
    request.payload.tasks[0].runtime = Some(TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    }));
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "true".into()],
        cwd: None,
        env: Default::default(),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload_digest = payload_digest(&request.payload).unwrap();

    assert_eq!(dispatch(&server, &request).await, 202);
    wait_terminal(&server, &request).await;
    let status = tak_runner::image_cache_status(&db_path, 1024 * 1024, 0.0, 0).unwrap();
    assert_eq!(status.entry_count, 1);
    assert!(
        snapshot(&server)
            .await
            .cached_content
            .contains(&"image:alpine:3.20".into())
    );
}
