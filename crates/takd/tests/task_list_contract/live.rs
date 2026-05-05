#![allow(clippy::await_holding_lock)]

use crate::support;

use prost::Message;
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use tak_proto::{SubmitTaskResponse, decode_remote_token};
use takd::agent::{InitAgentOptions, init_agent, read_token_wait};
use takd::serve_agent;

use support::env::{EnvGuard, env_lock};

#[tokio::test(flavor = "multi_thread")]
async fn tasks_lists_live_jobs_from_running_agent_control_socket() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = support::cli::roots(temp.path());
    init_agent(
        &config_root,
        &state_root,
        InitAgentOptions {
            node_id: Some("builder-direct"),
            display_name: None,
            transport: Some("direct"),
            base_url: Some("http://127.0.0.1:0"),
            pools: &[],
            tags: &[],
            capabilities: &[],
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init direct agent");

    let server = tokio::spawn({
        let config_root = config_root.clone();
        let state_root = state_root.clone();
        async move { serve_agent(&config_root, &state_root).await }
    });
    let token = tokio::task::spawn_blocking({
        let state_root = state_root.clone();
        move || read_token_wait(&state_root, 5)
    })
    .await
    .expect("join token wait")
    .expect("wait token");
    let payload = decode_remote_token(&token).expect("decode direct token");
    let node = payload.node.expect("token node");
    submit_sleep(&node.base_url, &payload.bearer_token).await;

    let stdout = wait_for_task_listing(&config_root, &state_root, "task-run-live").await;
    assert!(stdout.contains("builder-direct"), "missing node:\n{stdout}");
    assert!(stdout.contains("//apps/web:build"));
    assert!(stdout.contains("attempt=1"), "missing attempt:\n{stdout}");
    assert!(stdout.contains("runtime=containerized"));
    server.abort();
}

async fn submit_sleep(base_url: &str, bearer_token: &str) {
    let authority = base_url.strip_prefix("http://").expect("direct base url");
    let mut stream = std::net::TcpStream::connect(authority).expect("connect takd");
    let body =
        support::remote_v1_http_submit::submit_request("task-run-live", Vec::new()).encode_to_vec();
    write!(
        stream,
        "POST /v1/tasks/submit HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("write submit head");
    stream.write_all(&body).expect("write submit body");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("read submit response");
    let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let submit = SubmitTaskResponse::decode(&response[split..]).expect("decode submit");
    assert!(submit.accepted, "submit accepted");
}

async fn wait_for_task_listing(config_root: &Path, state_root: &Path, needle: &str) -> String {
    let mut last = String::new();
    for _ in 0..80 {
        let output = support::takd_tasks::run_takd_tasks(config_root, state_root);
        last = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if output.status.success() && last.contains(needle) {
            return last;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {needle} in takd tasks:\n{last}");
}
