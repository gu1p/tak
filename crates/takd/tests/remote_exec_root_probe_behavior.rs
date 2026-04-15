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
async fn containerized_remote_tasks_probe_and_choose_visible_tmpdir_root() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");
    let visible_root = tmpdir.join("takd-remote-exec");

    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![visible_root.clone()],
            image_present: false,
        },
    );
    configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env);
    env.remove("TAKD_REMOTE_EXEC_ROOT");
    env.set("TMPDIR", tmpdir.display().to_string());

    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-probed", "true");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-probed");

    let result = fetch_result(&context, &store, "task-run-probed");
    assert!(result.success);

    let creates = daemon.create_records();
    assert!(
        creates.iter().any(|record| record.is_probe()),
        "expected at least one probe container: {creates:?}"
    );
    let execution = creates
        .iter()
        .find(|record| !record.is_probe())
        .expect("execution container");
    assert!(
        execution
            .bind_source()
            .expect("execution bind source")
            .starts_with(&visible_root),
        "execution bind should use visible tmpdir root: {:?}",
        execution
    );
    assert_eq!(daemon.pull_count(), 1, "probe should pull alpine once");
}
