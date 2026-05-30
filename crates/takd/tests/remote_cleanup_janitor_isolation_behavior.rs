use std::collections::BTreeMap;
use std::time::Duration;

use takd::{RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_output::test_context_with_runtime,
};

/// A cleanup janitor whose node has no explicitly configured docker host must
/// not fall back to the process-global `DOCKER_HOST` env and reap containers
/// that belong to a *different* node's daemon. Under the parallel test suite
/// that env is shared, so this previously let one test's janitor connect to
/// another test's fake daemon and force-remove its still-active container — the
/// root cause of the CI-only orphan-watchdog flake. `for_tests()` now defaults
/// to an isolated docker host, so each node only ever touches its own daemon.
#[test]
fn cleanup_janitor_does_not_reap_active_containers_on_another_daemon() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp_other = tempfile::tempdir().expect("tempdir other");
    let temp_node = tempfile::tempdir().expect("tempdir node");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        // Another node's daemon, holding an active container it owns.
        let other_daemon = FakeDockerDaemon::spawn(temp_other.path(), FakeDockerConfig::default());
        other_daemon.add_container(
            "container-1",
            BTreeMap::from([
                ("tak.owner".to_string(), "takd".to_string()),
                ("tak.submit_key".to_string(), "other-job:1".to_string()),
            ]),
        );

        // Point the shared global docker host env at the other node's daemon.
        env.set(
            "DOCKER_HOST",
            format!("unix://{}", other_daemon.socket_path().display()),
        );

        // This node has no docker host of its own and no active execution for
        // "other-job:1"; its janitor must not reach the other daemon.
        let config =
            RemoteRuntimeConfig::for_tests().with_remote_cleanup_interval(Duration::from_millis(5));
        let context = test_context_with_runtime(config);
        let store =
            SubmitAttemptStore::with_db_path(temp_node.path().join("node.sqlite")).expect("store");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let server = tokio::spawn(run_remote_v1_http_server(
            listener,
            store.clone(),
            context.clone(),
        ));

        // Give several janitor sweeps a chance to (mis)fire.
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(
            other_daemon.removed_containers().is_empty(),
            "janitor reaped another daemon's active container: {:?}",
            other_daemon.removed_containers()
        );
        server.abort();
    });
}
