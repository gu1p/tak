use super::*;

/// A node upgraded to scoped ownership must be safe while another daemon on
/// the same Docker engine is still running the legacy global janitor. That
/// janitor selects exactly `tak.owner=takd`, so new containers must not carry
/// that legacy owner value.
#[test]
fn new_node_containers_are_invisible_to_the_legacy_global_janitor() {
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
                ..Default::default()
            },
        );
        let config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
            .with_explicit_remote_exec_root(exec_root)
            .with_skip_exec_root_probe(true)
            .build();
        let owner = test_context_for_node_with_runtime("builder-a", config);
        let owner_store = store(temp.path().join("owner.sqlite"));

        assert!(submit_container_task(&owner, &owner_store, "active-run", "true").accepted);
        let active = wait_for_created_container(&daemon).await;

        assert!(
            !legacy_global_janitor_selects(&active),
            "new container still matches the legacy tak.owner=takd selector"
        );
    });
}

fn legacy_global_janitor_selects(record: &CreateRecord) -> bool {
    record.labels.get("tak.owner").map(String::as_str) == Some("takd")
}
