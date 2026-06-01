use std::time::Duration;

use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{
    RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request, run_remote_v1_http_server,
};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
};

mod active_liveness;

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread")]
async fn watchdog_cancels_active_worker_without_client_heartbeat() {
    // The guard serializes process-env mutation for the whole async test body.
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            wait_response_delay: Duration::from_secs(30),
            ..FakeDockerConfig::default()
        },
    );
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(temp.path().join("remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_remote_client_stale_ttl(Duration::from_millis(200))
        .with_remote_client_watchdog_interval(Duration::from_millis(10));
    let context = test_context_with_runtime(runtime_config);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind server");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));

    let submit = submit_container_task(&context, &store, "task-run-orphan", "sleep 60");
    assert!(submit.accepted);

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while daemon.create_records().is_empty() && std::time::Instant::now() < deadline {
        get(&context, &store, "task-run-orphan", "events");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!daemon.create_records().is_empty(), "no container started");
    wait_for_removed_container(&daemon).await;

    let result = result(&context, &store, "task-run-orphan");
    assert!(!result.success);
    assert_eq!(result.status, "cancelled");
    assert!(
        result
            .stderr_tail
            .as_deref()
            .unwrap_or_default()
            .contains("without client contact"),
        "missing orphan diagnostic: {result:?}"
    );
    assert!(!daemon.removed_containers().is_empty());
    server.abort();
}

async fn wait_for_removed_container(daemon: &FakeDockerDaemon) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while daemon.removed_containers().is_empty() {
        assert!(
            std::time::Instant::now() < deadline,
            "container was not removed"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn get(context: &RemoteNodeContext, store: &SubmitAttemptStore, task: &str, endpoint: &str) {
    let path = format!("/v1/tasks/{task}/{endpoint}?attempt=1");
    handle_remote_v1_request(context, store, "GET", &path, None).expect("remote request");
}

fn result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task: &str,
) -> GetTaskResultResponse {
    let path = format!("/v1/tasks/{task}/result?attempt=1");
    let response =
        handle_remote_v1_request(context, store, "GET", &path, None).expect("result response");
    assert_eq!(response.status_code, 200);
    GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result")
}
