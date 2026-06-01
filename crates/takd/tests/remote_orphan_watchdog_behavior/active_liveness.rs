use std::time::{Duration, Instant};

use prost::Message;
use tak_proto::GetTaskResultResponse;
use takd::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request,
    run_remote_v1_http_server,
};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::{configure_fake_docker_env, submit_container_task},
    remote_output::test_context_with_runtime,
};

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread")]
async fn result_polling_keeps_active_execution_live() {
    // The guard serializes process-env mutation for the whole async test body.
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![temp.path().to_path_buf()],
            wait_response_delay: Duration::from_millis(900),
            ..FakeDockerConfig::default()
        },
    );
    let runtime = runtime_config(temp.path(), daemon.socket_path(), &mut env);
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind server");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    let submit = submit_container_task(&context, &store, "task-run-live", "printf ok");
    assert!(submit.accepted);

    let deadline = Instant::now() + Duration::from_millis(650);
    while Instant::now() < deadline {
        let _ = get(&context, &store, "task-run-live", "result");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let result = wait_for_result(&context, &store, "task-run-live").await;
    assert!(result.success, "live task was cancelled: {result:?}");
    server.abort();
}

fn runtime_config(
    root: &std::path::Path,
    socket_path: &std::path::Path,
    env: &mut EnvGuard,
) -> RemoteRuntimeConfig {
    configure_fake_docker_env(root, socket_path, env)
        .with_explicit_remote_exec_root(root.join("remote-exec"))
        .with_skip_exec_root_probe(true)
        .with_remote_client_stale_ttl(Duration::from_millis(200))
        .with_remote_client_watchdog_interval(Duration::from_millis(10))
}

fn get(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    endpoint: &str,
) -> takd::RemoteV1Response {
    let path = format!("/v1/tasks/{task_run_id}/{endpoint}?attempt=1");
    handle_remote_v1_request(context, store, "GET", &path, None).expect("remote request")
}

async fn wait_for_result(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
) -> GetTaskResultResponse {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let response = get(context, store, task_run_id, "result");
        if response.status_code == 200 {
            return GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result");
        }
        assert!(Instant::now() < deadline, "result timed out");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
