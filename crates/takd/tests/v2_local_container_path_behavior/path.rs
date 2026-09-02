use super::*;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn image_path_is_not_host_path_unless_explicitly_passed() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().unwrap();
    let docker = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![temp.path().to_path_buf()],
            ..Default::default()
        },
    );
    let _runtime = configure_fake_docker_env(temp.path(), docker.socket_path(), &mut env).build();
    let host_path = std::env::var("PATH").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = temp.path().join("takd.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    assert!(wait_for(|| socket.exists()).await);
    let store = RunStore::with_db_path(db).unwrap();
    commit_and_wait(&store, container_run("image-path", None)).await;
    commit_and_wait(&store, container_run("passed-path", Some("/client/bin"))).await;

    let creates = docker.create_records();
    assert_eq!(creates.len(), 2, "unexpected containers: {creates:?}");
    assert!(!creates[0].env.contains(&format!("PATH={host_path}")));
    assert!(creates[1].env.contains(&"PATH=/client/bin".to_string()));
    server.abort();
}
