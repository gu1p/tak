use std::collections::BTreeMap;

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn measured_tak_usage_excludes_another_node_on_the_same_engine() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let measured_bytes = 4 * 1024 * 1024;
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            memory_usage_bytes: measured_bytes,
            ..Default::default()
        },
    );
    let labels = |node_id: &str, submit_key: &str| {
        BTreeMap::from([
            ("tak.owner".to_string(), "takd-node-v1".to_string()),
            ("tak.node_id".to_string(), node_id.to_string()),
            ("tak.submit_key".to_string(), submit_key.to_string()),
        ])
    };
    // Paused containers remain measurable but are protected from the cleanup
    // sweep that starts alongside the usage sampler.
    daemon.add_paused_container("owned", labels("builder-a", "owned:1"));
    daemon.add_paused_container("foreign", labels("builder-b", "foreign:1"));
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
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

    let deadline = Instant::now() + Duration::from_secs(3);
    let sampled_usage = loop {
        let usage = status(&context, &store)
            .resource_envelope
            .map(|envelope| envelope.tak_usage_memory_bytes)
            .unwrap_or_default();
        if usage > 0 {
            break usage;
        }
        assert!(Instant::now() < deadline, "usage sample was not published");
        tokio::time::sleep(Duration::from_millis(10)).await;
    };

    assert_eq!(sampled_usage, measured_bytes);
    server.abort();
}
