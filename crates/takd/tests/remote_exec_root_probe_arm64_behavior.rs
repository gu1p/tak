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
async fn containerized_remote_tasks_choose_arm64_probe_helper_for_arm64_daemon() {
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
            arch: "arm64".to_string(),
        },
    );
    configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env);
    env.remove("TAKD_REMOTE_EXEC_ROOT");
    env.set("TMPDIR", tmpdir.display().to_string());

    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-probed-arm64", "true");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-probed-arm64");

    let result = fetch_result(&context, &store, "task-run-probed-arm64");
    assert!(result.success);

    let creates = daemon.create_records();
    let execution = creates
        .iter()
        .find(|record| !record.is_probe())
        .expect("execution container");
    let probe = creates
        .iter()
        .find(|record| record.is_probe())
        .expect("probe container");
    assert_eq!(
        probe.image.as_deref(),
        Some("takd-exec-root-probe:aarch64-v1"),
        "probe should follow the daemon architecture instead of the takd build architecture"
    );
    assert_ne!(probe.image, execution.image);
    assert_eq!(daemon.pull_count(), 0);
}
