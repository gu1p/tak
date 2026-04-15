use std::fs;
use std::path::Path;

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
async fn containerized_remote_tasks_fall_back_to_unix_default_when_probe_cannot_validate_candidates()
 {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");

    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: Vec::new(),
            image_present: true,
        },
    );
    configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env);
    env.remove("TAKD_REMOTE_EXEC_ROOT");
    env.set("TMPDIR", tmpdir.display().to_string());

    let context = test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let ack = submit_container_task(&context, &store, "task-run-fallback", "true");
    assert!(ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-fallback");

    let result = fetch_result(&context, &store, "task-run-fallback");
    assert!(
        !result.success,
        "fallback root should remain unusable in fake docker"
    );

    let creates = daemon.create_records();
    assert!(
        creates.iter().any(|record| record.is_probe()),
        "expected probe attempts before fallback: {creates:?}"
    );
    let execution = creates
        .iter()
        .find(|record| !record.is_probe())
        .expect("execution container");
    for probe in creates.iter().filter(|record| record.is_probe()) {
        assert_ne!(
            probe.image, execution.image,
            "probe should use a dedicated helper image instead of the task runtime image"
        );
    }
    assert!(
        execution
            .bind_source()
            .expect("execution bind source")
            .starts_with(Path::new("/var/tmp/takd-remote-exec")),
        "execution bind should fall back to unix default root: {:?}",
        execution
    );
    assert_eq!(
        daemon.pull_count(),
        0,
        "probe fallback should not need any registry pull when the task image is already present"
    );
}
