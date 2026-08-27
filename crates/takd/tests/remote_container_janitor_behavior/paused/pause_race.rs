use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn janitor_reinspects_before_force_removing_a_newly_paused_container() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let daemon = FakeDockerDaemon::spawn(temp.path(), FakeDockerConfig::default());
    daemon.add_container("pause-race", takd_labels("stale-pausing:1"));
    daemon.pause_container_after_next_list("pause-race");
    daemon.add_container("sweep-marker", takd_labels("stale-marker:1"));
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_interval(Duration::from_millis(10))
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

    wait_for_removed(&daemon, "sweep-marker").await;
    sleep(Duration::from_millis(80)).await;
    assert!(!daemon.removed_containers().contains(&"pause-race".into()));
    server.abort();
    let _ = server.await;
}
