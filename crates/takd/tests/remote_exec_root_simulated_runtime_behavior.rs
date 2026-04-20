use takd::SubmitAttemptStore;

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use support::wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn simulated_container_runtime_skips_exec_root_probe() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path(), FakeDockerConfig::default());

    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_skip_exec_root_probe(true);
    let context = support::remote_output::test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-simulated", "true");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-simulated");

    let result = fetch_result(&context, &store, "task-run-simulated");
    assert!(result.success);
    assert_eq!(result.runtime.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));
    assert!(
        daemon.create_records().is_empty(),
        "simulated container runtime should not probe docker"
    );
}
