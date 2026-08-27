#![allow(clippy::await_holding_lock)]

use std::time::{Duration, Instant};

use tak_proto::ContainerResourceLimits;
use takd::SubmitAttemptStore;
use takd::daemon::remote::run_remote_v1_http_server;
use tokio::net::TcpListener;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

use super::{status, submit};

#[path = "live_usage/paused.rs"]
mod paused;

#[tokio::test(flavor = "multi_thread")]
async fn measured_tak_usage_queues_new_elastic_work() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            image_present: true,
            wait_response_delay: Duration::from_secs(10),
            memory_usage_bytes: u64::MAX / 4,
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(tmpdir.join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    submit(
        &context,
        &store,
        "existing-work",
        "sleep 60",
        Some(ContainerResourceLimits {
            cpu_cores: 0.1,
            memory_mb: 1,
        }),
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let published_usage = status(&context, &store)
            .resource_envelope
            .as_ref()
            .map(|envelope| envelope.tak_usage_memory_bytes)
            .unwrap_or_default();
        if published_usage > 0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "usage sampler did not publish the completed Docker stats sample"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    submit(&context, &store, "elastic-next", "sleep 1", None);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let snapshot = status(&context, &store);
    assert!(
        snapshot
            .queued_jobs
            .iter()
            .any(|job| job.task_run_id == "elastic-next"),
        "new work should queue behind measured pressure: {snapshot:?}"
    );
    server.abort();
}
