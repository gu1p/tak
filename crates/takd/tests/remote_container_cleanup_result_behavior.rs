#![allow(clippy::await_holding_lock)]

use std::fs;

use takd::SubmitAttemptStore;

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, fetch_result, submit_container_task},
    remote_output::test_context_with_runtime,
    wait_for_terminal_events::wait_for_terminal_events,
};

#[tokio::test(flavor = "multi_thread")]
async fn completed_container_result_survives_unconfirmed_cleanup() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    fs::create_dir_all(&exec_root).expect("exec root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            removal_failures: 1,
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let submit = submit_container_task(&context, &store, "cleanup-result", "true");
    assert!(submit.accepted);
    wait_for_terminal_events(&context, &store, "cleanup-result");
    let result = fetch_result(&context, &store, "cleanup-result");

    assert!(
        result.success,
        "cleanup failure discarded the completed result"
    );
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.failure_kind, None);
}
