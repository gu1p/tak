#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_proto::RemoteFailureKind;
use takd::SubmitAttemptStore;

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{
        configure_fake_docker_env, fetch_result, submit_fused_container_task_with_retry,
    },
    remote_output::test_context_with_runtime,
    wait_for_terminal_events::wait_for_terminal_events,
};

#[tokio::test(flavor = "multi_thread")]
async fn confirmed_fused_member_oom_bypasses_authored_same_worker_retries() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    fs::create_dir_all(&exec_root).expect("exec root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            oom_killed: true,
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root)
        .with_skip_exec_root_probe(true)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let submit =
        submit_fused_container_task_with_retry(&context, &store, "fused-oom", "exit 137", 3);
    assert!(submit.accepted);
    wait_for_terminal_events(&context, &store, "fused-oom");
    let result = fetch_result(&context, &store, "fused-oom");
    let member_runs = daemon
        .create_records()
        .into_iter()
        .filter(|record| !record.is_probe())
        .count();

    assert_eq!(member_runs, 1, "confirmed OOM consumed authored retries");
    assert_eq!(result.exit_code, Some(137));
    assert_eq!(
        result.failure_kind,
        Some(RemoteFailureKind::ContainerOom as i32)
    );
}
