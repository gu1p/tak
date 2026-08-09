use std::time::Duration;

use takd::{SubmitAttemptStore, run_remote_v1_http_server};
use tokio::{net::TcpListener, time::sleep};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::configure_fake_docker_env,
    remote_output::test_context_with_runtime,
};

use super::{takd_labels, wait_for_removed};

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
        .with_remote_cleanup_interval(Duration::from_millis(10))
        .build();
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

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
