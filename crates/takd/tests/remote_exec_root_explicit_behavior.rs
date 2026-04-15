use std::fs;

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
async fn explicit_remote_exec_root_skips_probe_for_containerized_remote_tasks() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let explicit_root = temp.path().join("explicit-exec-root");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");

    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![explicit_root.clone()],
            image_present: false,
        },
    );
    configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env);
    env.set("TAKD_REMOTE_EXEC_ROOT", explicit_root.display().to_string());
    env.set("TMPDIR", tmpdir.display().to_string());

    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-explicit", "true");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-explicit");

    let result = fetch_result(&context, &store, "task-run-explicit");
    assert!(result.success);
    assert_eq!(result.runtime.as_deref(), Some("containerized"));
    assert_eq!(result.runtime_engine.as_deref(), Some("docker"));

    let creates = daemon.create_records();
    assert_eq!(
        creates.len(),
        1,
        "explicit root should skip probe containers"
    );
    assert!(!creates[0].is_probe(), "explicit root should not probe");
    assert!(
        creates[0]
            .bind_source()
            .expect("execution bind source")
            .starts_with(&explicit_root),
        "execution bind should use explicit root: {:?}",
        creates[0]
    );
}
