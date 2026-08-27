use std::fs;

use takd::SubmitAttemptStore;

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use support::remote_output::test_context_with_runtime;
use support::wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn unattributed_exit_137_persists_legacy_task_kind_and_truthful_diagnostic() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("tmpdir");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.join("takd-remote-exec")],
            image_present: true,
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    submit_container_task(&context, &store, "task-run-137", "exit 137");
    wait_for_terminal_events(&context, &store, "task-run-137");
    let result = fetch_result(&context, &store, "task-run-137");

    assert_eq!(result.exit_code, Some(137));
    assert_eq!(
        result.failure_kind,
        Some(tak_proto::RemoteFailureKind::Task as i32)
    );
    assert!(
        result
            .stderr_tail
            .as_deref()
            .is_some_and(|tail| tail.contains("exit code 137")
                && tail.contains("OOMKilled=false")
                && tail.contains("cause is unknown"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn engine_confirmed_container_oom_persists_distinct_wire_kind() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("tmpdir");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.join("takd-remote-exec")],
            image_present: true,
            oom_killed: true,
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    submit_container_task(&context, &store, "task-run-oom", "exit 137");
    wait_for_terminal_events(&context, &store, "task-run-oom");
    let result = fetch_result(&context, &store, "task-run-oom");

    assert_eq!(result.exit_code, Some(137));
    assert_eq!(
        result.failure_kind,
        Some(tak_proto::RemoteFailureKind::ContainerOom as i32)
    );
}
