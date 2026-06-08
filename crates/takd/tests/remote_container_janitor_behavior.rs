#![allow(clippy::await_holding_lock)]

use std::{collections::BTreeMap, time::Duration};

use takd::{SubmitAttemptStore, build_submit_idempotency_key, run_remote_v1_http_server};
use tokio::{net::TcpListener, time::sleep};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
};

#[tokio::test(flavor = "multi_thread")]
async fn container_janitor_removes_inactive_takd_containers_only() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    daemon.add_container("leaked-container", takd_labels("leaked-run:1"));
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_interval(Duration::from_millis(10));
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    let submit = submit_container_task(&context, &store, "active-run", "sleep 60");
    assert!(submit.accepted);
    let active = wait_for_active_container(&daemon).await;
    wait_for_removed(&daemon, "leaked-container").await;
    sleep(Duration::from_millis(80)).await;

    assert!(!daemon.removed_containers().contains(&active));
    server.abort();
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread")]
async fn container_janitor_never_removes_paused_containers() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            ..Default::default()
        },
    );
    // A running, inactive container is a legitimate orphan and should be reaped...
    daemon.add_container("running-leaked", takd_labels("running-leaked:1"));
    // ...but a paused container must NEVER be force-removed: pausing is the
    // memory-pressure controller's non-lethal hold, so reaping it would turn a
    // pause into a kill (e.g. after a daemon restart with an empty active set).
    daemon.add_paused_container("frozen-container", takd_labels("frozen-run:1"));
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_interval(Duration::from_millis(10));
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    // Once the running orphan is reaped, a sweep has completed; the paused
    // container must have survived it.
    wait_for_removed(&daemon, "running-leaked").await;
    sleep(Duration::from_millis(80)).await;
    assert!(
        !daemon
            .removed_containers()
            .contains(&"frozen-container".to_string()),
        "paused container must never be force-removed: {:?}",
        daemon.removed_containers()
    );
    server.abort();
    let _ = server.await;
}

fn takd_labels(submit_key: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("tak.owner".to_string(), "takd".to_string()),
        ("tak.submit_key".to_string(), submit_key.to_string()),
    ])
}

async fn wait_for_active_container(daemon: &FakeDockerDaemon) -> String {
    let active_key = build_submit_idempotency_key("active-run", Some(1)).expect("key");
    for _ in 0..250 {
        if let Some(record) = daemon
            .create_records()
            .into_iter()
            .find(|record| record.labels.get("tak.submit_key") == Some(&active_key))
        {
            return record.container_id;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for active takd container");
}

async fn wait_for_removed(daemon: &FakeDockerDaemon, container_id: &str) {
    for _ in 0..250 {
        if daemon
            .removed_containers()
            .iter()
            .any(|removed| removed == container_id)
        {
            return;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for removal of {container_id}");
}
