use std::time::Duration;

use takd::{SubmitAttemptStore, run_remote_v1_http_server};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{CreateRecord, FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_for_node_with_runtime,
};

/// Two independently configured Tak nodes may intentionally share one Docker
/// engine. A submit key is only locally meaningful, so each janitor must
/// restrict itself to containers carrying its own node identity.
#[test]
fn cleanup_janitor_does_not_reap_another_nodes_active_container_on_shared_daemon() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        let daemon = FakeDockerDaemon::spawn(
            temp.path(),
            FakeDockerConfig {
                visible_roots: vec![exec_root.clone()],
                wait_response_delay: Duration::from_secs(30),
                ..Default::default()
            },
        );
        let config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
            .with_explicit_remote_exec_root(exec_root)
            .with_skip_exec_root_probe(true)
            .with_remote_cleanup_interval(Duration::from_millis(5))
            .build();
        let owner = test_context_for_node_with_runtime("builder-a", config.clone());
        let owner_store = store(temp.path().join("owner.sqlite"));
        assert!(submit_container_task(&owner, &owner_store, "active-run", "sleep 60").accepted);
        let active = wait_for_created_container(&daemon).await;

        let other = test_context_for_node_with_runtime("builder-b", config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let server = tokio::spawn(run_remote_v1_http_server(
            listener,
            store(temp.path().join("other.sqlite")),
            other,
        ));

        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(!daemon.removed_containers().contains(&active.container_id));
        assert_eq!(
            active.labels.get("tak.node_id").map(String::as_str),
            Some("builder-a")
        );
        server.abort();
        let _ = server.await;
    });
}

fn store(path: std::path::PathBuf) -> SubmitAttemptStore {
    SubmitAttemptStore::with_db_path(path).expect("store")
}

async fn wait_for_created_container(daemon: &FakeDockerDaemon) -> CreateRecord {
    for _ in 0..250 {
        if let Some(record) = daemon
            .create_records()
            .into_iter()
            .find(|record| record.labels.contains_key("tak.submit_key"))
        {
            return record;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for the active remote container");
}
