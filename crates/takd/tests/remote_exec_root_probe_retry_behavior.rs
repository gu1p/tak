use std::fs;

use takd::SubmitAttemptStore;

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::{configure_fake_docker_env, fetch_result, submit_container_task};
use support::remote_output::test_context_with_runtime;
use support::wait_for_terminal_events::wait_for_terminal_events;

#[tokio::test(flavor = "multi_thread")]
async fn containerized_remote_tasks_retry_probe_after_transient_probe_failure() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmpdir");
    let visible_root = tmpdir.join("takd-remote-exec");
    let socket_path = temp.path().join("docker.sock");

    let runtime_config =
        configure_fake_docker_env(temp.path(), &socket_path, &mut env).with_temp_dir(tmpdir);
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let first_ack = submit_container_task(&context, &store, "task-run-probe-retry-1", "true");
    assert!(first_ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-probe-retry-1");

    let first_result = fetch_result(&context, &store, "task-run-probe-retry-1");
    assert!(
        !first_result.success,
        "first submit should fail while the docker daemon is unavailable"
    );

    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![visible_root.clone()],
            image_present: true,
            ..Default::default()
        },
    );

    let second_ack = submit_container_task(&context, &store, "task-run-probe-retry-2", "true");
    assert!(second_ack.accepted);
    wait_for_terminal_events(&context, &store, "task-run-probe-retry-2");

    let second_result = fetch_result(&context, &store, "task-run-probe-retry-2");
    assert!(
        second_result.success,
        "second submit should succeed after the daemon becomes available"
    );

    let creates = daemon.create_records();
    assert!(
        creates.iter().any(|record| record.is_probe()),
        "expected the second submit to retry the exec-root probe: {creates:?}"
    );
    let execution = creates
        .iter()
        .find(|record| !record.is_probe())
        .expect("execution container");
    let probe = creates
        .iter()
        .find(|record| record.is_probe())
        .expect("probe container");
    assert!(
        execution
            .bind_source()
            .expect("execution bind source")
            .starts_with(&visible_root),
        "execution bind should use the visible tmpdir root after the retry: {:?}",
        execution
    );
    assert_ne!(
        probe.image, execution.image,
        "probe should use a dedicated helper image instead of the task runtime image"
    );
    assert_eq!(
        daemon.pull_count(),
        0,
        "retried probe should not need any registry pull when the task image is already present"
    );
}
