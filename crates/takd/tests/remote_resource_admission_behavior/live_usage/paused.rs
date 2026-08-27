#![allow(clippy::await_holding_lock)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use takd::SubmitAttemptStore;
use takd::daemon::remote::run_remote_v1_http_server;
use tokio::net::TcpListener;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

use super::status;

#[tokio::test(flavor = "multi_thread")]
async fn paused_tak_container_remains_in_usage_attribution() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let measured_bytes = 4 * 1024 * 1024;
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            memory_usage_bytes: measured_bytes,
            ..Default::default()
        },
    );
    daemon.add_paused_container(
        "paused-tak",
        BTreeMap::from([
            ("tak.owner".into(), "takd".into()),
            ("tak.submit_key".into(), "paused-run:1".into()),
        ]),
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

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let usage = status(&context, &store)
            .resource_envelope
            .map(|envelope| envelope.tak_usage_memory_bytes)
            .unwrap_or_default();
        if usage >= measured_bytes {
            break;
        }
        assert!(Instant::now() < deadline, "paused Tak usage was omitted");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    server.abort();
}
