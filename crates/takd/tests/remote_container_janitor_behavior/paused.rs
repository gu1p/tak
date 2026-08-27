use std::time::Duration;

use takd::{SubmitAttemptStore, build_submit_idempotency_key, run_remote_v1_http_server};
use tokio::{net::TcpListener, time::sleep};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
};

use super::{takd_labels, wait_for_removed};

#[path = "paused/pause_race.rs"]
mod pause_race;

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
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    daemon.add_paused_container("previous-daemon-paused", takd_labels("stale-run:1"));
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_interval(Duration::from_millis(10))
        .build();
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let submit = submit_container_task(&context, &store, "active-run", "sleep 60");
    assert!(submit.accepted);
    let active_key = build_submit_idempotency_key("active-run", Some(1)).expect("key");
    daemon.add_paused_container("active-paused", takd_labels(&active_key));
    daemon.add_container("running-leaked", takd_labels("running-leaked:1"));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

    wait_for_removed(&daemon, "running-leaked").await;
    sleep(Duration::from_millis(80)).await;
    let removed = daemon.removed_containers();
    assert!(!removed.contains(&"previous-daemon-paused".to_string()));
    assert!(!removed.contains(&"active-paused".to_string()));
    server.abort();
    let _ = server.await;
}
