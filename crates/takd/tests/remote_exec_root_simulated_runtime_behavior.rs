use takd::SubmitAttemptStore;

#[path = "support/remote_container.rs"]
mod remote_container;
#[path = "support/remote_output.rs"]
mod remote_output;
mod support;
#[path = "support/wait_for_terminal_events.rs"]
mod wait_for_terminal_events;

use remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use remote_output::test_context;
use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn simulated_container_runtime_skips_exec_root_probe() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path(), FakeDockerConfig::default());

    configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env);
    env.remove("TAKD_REMOTE_EXEC_ROOT");
    env.set("TAK_TEST_HOST_PLATFORM", "other");

    let context = test_context();
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
