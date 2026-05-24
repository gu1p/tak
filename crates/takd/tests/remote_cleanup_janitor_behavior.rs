#![allow(clippy::await_holding_lock)]

use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use takd::{SubmitAttemptStore, build_submit_idempotency_key, run_remote_v1_http_server};
use tokio::{net::TcpListener, time::sleep};

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::configure_fake_docker_env;
use support::remote_output::{submit_shell_task, test_context_with_runtime};

const REMOTE_WORKER_STATE_TIMEOUT: Duration = Duration::from_secs(45);

#[tokio::test(flavor = "multi_thread")]
async fn cleanup_janitor_removes_stale_roots_but_preserves_active_jobs() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let artifact_root = temp.path().join("takd-remote-artifacts");
    fs::create_dir_all(&exec_root).expect("create exec root");
    fs::create_dir_all(&artifact_root).expect("create artifact root");

    let stale_exec_root = exec_root.join("stale-job_1");
    let stale_artifact_root = artifact_root.join("stale-job_1");
    fs::create_dir_all(&stale_exec_root).expect("create stale exec root");
    fs::create_dir_all(&stale_artifact_root).expect("create stale artifact root");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind remote listener");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root.clone())
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_ttl(Duration::from_millis(10))
        .with_remote_cleanup_interval(Duration::from_millis(10));
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    let submit_ack = submit_shell_task(&context, &store, "active-job", "sleep 30");
    assert!(submit_ack.accepted);

    let active_key = build_submit_idempotency_key("active-job", Some(1)).expect("active key");
    let active_root_name = active_key.replace(':', "_");
    let active_exec_root = exec_root.join(&active_root_name);
    let active_artifact_root = artifact_root.join(&active_root_name);
    fs::create_dir_all(&active_artifact_root).expect("create active artifact root");

    wait_for_path(&active_exec_root, true, "creation").await;
    wait_for_path(&stale_exec_root, false, "cleanup").await;
    wait_for_path(&stale_artifact_root, false, "cleanup").await;

    assert!(active_exec_root.exists());
    assert!(active_artifact_root.exists());

    server.abort();
    let _ = server.await;
}

async fn wait_for_path(path: &Path, expected_present: bool, action: &str) {
    let deadline = Instant::now() + REMOTE_WORKER_STATE_TIMEOUT;
    loop {
        if path.exists() == expected_present {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {action} of {}",
            path.display()
        );
        sleep(Duration::from_millis(20)).await;
    }
}
