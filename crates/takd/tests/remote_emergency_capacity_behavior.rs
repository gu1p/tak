#![allow(clippy::await_holding_lock)]

use std::fs;
use std::time::Duration;

use takd::SubmitAttemptStore;

use crate::support::env::{EnvGuard, env_lock};
use crate::support::fake_docker_daemon::{DockerOperation, FakeDockerConfig, FakeDockerDaemon};
use crate::support::http::fetch_node_status;
use crate::support::pressure as behavior;
use crate::support::remote_container::{configure_fake_docker_env, submit_container_task};
use crate::support::remote_output::test_context_with_runtime;
use crate::support::synthetic_memory_signal::SyntheticMemorySignal;

#[tokio::test(flavor = "multi_thread")]
async fn emergency_holds_admission_and_advertises_zero_capacity() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let tmpdir = temp.path().join("tmp-root");
    fs::create_dir_all(&tmpdir).expect("create tmp root");
    let memory = SyntheticMemorySignal::healthy(temp.path());
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![tmpdir.clone()],
            wait_response_delay: Duration::from_secs(30),
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(tmpdir.join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_test_memory_signal(memory.path(), Duration::from_millis(20))
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    submit_container_task(&context, &store, "running", "sleep 60");
    behavior::wait_for_task_creates(&daemon, "running", 1);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener");
    let address = listener.local_addr().expect("listener address").to_string();
    let server = tokio::spawn(takd::run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    memory.apply_pressure();
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        daemon
            .operations()
            .iter()
            .all(|operation| !matches!(operation, DockerOperation::RemovalAttempted(_))),
        "memory pressure must never force-remove running work"
    );

    submit_container_task(&context, &store, "queued", "printf queued");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        daemon.create_records().iter().all(|record| {
            record.labels.get("tak.task_run_id").map(String::as_str) != Some("queued")
        }),
        "emergency pressure must hold new admission"
    );
    let status = fetch_node_status(&address, "builder", "secret").await;
    let envelope = status.resource_envelope.expect("resource envelope");
    let cpu = status.cpu.expect("cpu status");
    let memory = status.memory.expect("memory status");
    assert_eq!(envelope.admittable_cpu_cores, 0.0);
    assert_eq!(envelope.admittable_memory_bytes, 0);
    assert_eq!(cpu.tak_admission_available_cores, Some(0.0));
    assert_eq!(memory.tak_admission_available_bytes, Some(0));
    server.abort();
}
