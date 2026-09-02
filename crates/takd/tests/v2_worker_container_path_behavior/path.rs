use super::*;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn image_path_is_not_host_path_unless_passed_or_overridden() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().unwrap();
    let exec_root = temp.path().join("exec");
    let docker = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), docker.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .build();
    let host_path = std::env::var("PATH").unwrap();
    let server = start_server_with_runtime(runtime).await;
    dispatch(&server, request("image-path", None, None)).await;
    dispatch(&server, request("passed-path", Some("/client/bin"), None)).await;
    dispatch(&server, request("step-path", None, Some("/step/bin"))).await;

    let creates = docker.create_records();
    assert_eq!(creates.len(), 3, "unexpected containers: {creates:?}");
    assert!(!creates[0].env.contains(&format!("PATH={host_path}")));
    assert!(creates[1].env.contains(&"PATH=/client/bin".to_string()));
    assert!(creates[2].env.contains(&"PATH=/step/bin".to_string()));
}
