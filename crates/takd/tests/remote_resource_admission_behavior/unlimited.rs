use std::time::{Duration, Instant};

use takd::SubmitAttemptStore;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

use super::{submit, wait_for_status};

#[tokio::test(flavor = "multi_thread")]
async fn remote_container_without_authored_resources_stays_elastic() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    env.set("TAKD_DEFAULT_CONTAINER_CPU_CORES", "1.25");
    env.set("TAKD_DEFAULT_CONTAINER_MEMORY_MB", "768");
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            image_present: true,
            wait_response_delay: Duration::from_secs(2),
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(tmpdir.join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_default_container_resources(1.25, 768)
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    submit(&context, &store, "unlimited", "sleep 1", None);

    let snapshot = wait_for_status(&context, &store, |value| !value.active_jobs.is_empty());
    assert_eq!(snapshot.active_jobs[0].task_run_id, "unlimited");
    assert!(
        snapshot.active_jobs[0].resource_limits.is_none(),
        "worker startup estimates must not become authored task reservations"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    let create = loop {
        if let Some(create) = daemon
            .create_records()
            .into_iter()
            .find(|record| !record.is_probe())
        {
            break create;
        }
        assert!(
            Instant::now() < deadline,
            "worker container was not created"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(
        create.nano_cpus, None,
        "an elastic task must be free to burst within the worker envelope"
    );
}
