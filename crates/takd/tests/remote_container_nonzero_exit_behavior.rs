use std::fs;

use takd::SubmitAttemptStore;

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use support::remote_output::test_context_with_runtime;
use support::wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn containerized_remote_task_nonzero_exit_stays_a_task_failure() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");
    let visible_root = tmpdir.join("takd-remote-exec");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![visible_root],
            image_present: true,
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir);
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-nonzero-exit", "exit 1");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-nonzero-exit");

    let result = fetch_result(&context, &store, "task-run-nonzero-exit");
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(1));
    assert_eq!(result.runtime.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
}
