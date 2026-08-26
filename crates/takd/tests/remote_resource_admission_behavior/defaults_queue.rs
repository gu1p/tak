use std::time::Duration;

use takd::SubmitAttemptStore;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

use super::{submit, wait_for_status};

#[tokio::test(flavor = "multi_thread")]
async fn aggregate_worker_defaults_queue_at_safe_capacity() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            image_present: true,
            wait_response_delay: Duration::from_secs(10),
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(tmpdir.join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_default_container_resources(256.0, u64::MAX)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    submit(&context, &store, "default-1", "sleep 1", None);
    submit(&context, &store, "default-2", "sleep 1", None);

    let status = wait_for_status(&context, &store, |status| {
        status.active_jobs.len() == 1 && status.queued_jobs.len() == 1
    });
    assert_eq!(status.active_jobs[0].task_run_id, "default-1");
    assert_eq!(status.queued_jobs[0].task_run_id, "default-2");
}
