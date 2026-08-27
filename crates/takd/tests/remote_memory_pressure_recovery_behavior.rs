#![allow(clippy::await_holding_lock)]

use std::time::Duration;

use takd::{SubmitAttemptStore, run_remote_v1_http_server};
use tokio::time::sleep;

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    http::fetch_node_status,
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
    synthetic_memory_signal::SyntheticMemorySignal,
};

#[path = "remote_memory_pressure_recovery_support.rs"]
mod recovery_support;
use recovery_support::{takd_labels, wait_for_unpause_attempts};

#[tokio::test(flavor = "multi_thread")]
async fn existing_paused_container_with_failed_unpause_keeps_admission_held() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let memory = SyntheticMemorySignal::healthy(temp.path());
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    daemon.add_paused_container("paused", takd_labels("existing-submit-key"));
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .with_test_memory_signal(memory.path(), Duration::from_millis(20))
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("listener address").to_string();
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    wait_for_unpause_attempts(&daemon, 2).await;
    let status = fetch_node_status(&address, "builder", "secret").await;
    let pressure = status.resource_pressure.expect("resource pressure status");
    assert_ne!(
        pressure.state, "healthy",
        "failed recovery must remain visible"
    );
    assert!(submit_container_task(&context, &store, "queued", "printf queued").accepted);
    sleep(Duration::from_millis(100)).await;
    let queued_started = daemon
        .create_records()
        .iter()
        .any(|record| record.labels.get("tak.task_run_id").map(String::as_str) == Some("queued"));
    assert!(!queued_started, "failed unpause released admission");
    server.abort();
    let _ = server.await;
}
