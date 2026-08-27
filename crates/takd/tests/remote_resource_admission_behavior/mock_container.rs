#![allow(clippy::await_holding_lock)]

use std::time::{Duration, Instant};

use takd::SubmitAttemptStore;
use takd::daemon::remote::run_remote_v1_http_server;
use tokio::net::TcpListener;

use crate::support::remote_output::test_context_with_runtime;

use super::{status, submit, task_events};

#[tokio::test(flavor = "multi_thread")]
async fn idle_mock_container_node_starts_an_accepted_submit() {
    let _env_lock = crate::support::env::env_lock();
    let mut env = crate::support::env::EnvGuard::default();
    env.set("MOCK_CONTAINER", "true");
    let temp = tempfile::tempdir().expect("tempdir");
    let runtime = crate::support::runtime_config::builder()
        .with_explicit_remote_exec_root(temp.path().join("takd-remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_default_container_resources(0.1, 1)
        .with_real_host_usage()
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");

    submit(&context, &store, "idle-mock-work", "true", None);
    let queued = status(&context, &store);
    assert!(queued.active_jobs.is_empty());
    assert_eq!(queued.queued_jobs.len(), 1);

    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    let discoverable = super::wait_for_status(&context, &store, |snapshot| {
        snapshot.resource_envelope.as_ref().is_some_and(|envelope| {
            envelope.admittable_cpu_cores > 0.0 && envelope.admittable_memory_bytes > 0
        })
    });
    assert!(
        discoverable.resource_envelope.is_some(),
        "an idle mock node should publish admission capacity"
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let events = task_events(&context, &store, "idle-mock-work");
        if events.iter().any(|event| event.kind == "TASK_COMPLETED") {
            break;
        }
        if Instant::now() >= deadline {
            let snapshot = status(&context, &store);
            panic!(
                "accepted work remained queued while the node had zero active jobs: events={events:?} status={snapshot:?}"
            );
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    server.abort();
}
