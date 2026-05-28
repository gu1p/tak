use std::fs;
use std::time::Duration;

use tak_proto::ContainerResourceLimits;
use takd::SubmitAttemptStore;

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{
    configure_fake_docker_env, fetch_result, submit_container_task_with_limits,
};
use support::remote_output::test_context_with_runtime;
use support::wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn remote_worker_result_duration_covers_actual_execution_time() {
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
            wait_response_delay: Duration::from_millis(150),
            ..Default::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_temp_dir(tmpdir);
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let ack = submit_container_task_with_limits(
        &context,
        &store,
        "task-run-duration",
        "true",
        ContainerResourceLimits {
            cpu_cores: 0.001,
            memory_mb: 1,
        },
    );
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-duration");

    let result = fetch_result(&context, &store, "task-run-duration");
    assert!(
        result.duration_ms >= 100,
        "duration should include remote execution wait, got {}ms",
        result.duration_ms
    );
    assert!(result.finished_at >= result.started_at);
}
