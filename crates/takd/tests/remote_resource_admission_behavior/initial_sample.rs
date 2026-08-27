#![allow(clippy::await_holding_lock)]

use std::time::Duration;

use takd::SubmitAttemptStore;
use takd::daemon::remote::run_remote_v1_http_server;
use tokio::net::TcpListener;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

use super::{status, submit};

#[tokio::test(flavor = "multi_thread")]
async fn a_submit_waits_for_the_initial_live_host_sample() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            ping_response_delay: Duration::from_millis(500),
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(tmpdir.join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_real_host_usage()
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    submit(&context, &store, "startup-work", "true", None);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let snapshot = status(&context, &store);
    assert!(
        snapshot
            .queued_jobs
            .iter()
            .any(|job| job.task_run_id == "startup-work"),
        "work must remain queued before the first live host sample: {snapshot:?}"
    );
    assert!(daemon.create_records().is_empty());
    server.abort();
}
