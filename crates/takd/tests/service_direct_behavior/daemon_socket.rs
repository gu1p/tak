#![allow(clippy::await_holding_lock)]

use tak_proto::decode_remote_token;
use takd::agent::{InitAgentOptions, init_agent, read_token_wait};
use takd::{Request, Response, StatusRequest, serve_agent};

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::http::fetch_node_info;
use support::protocol::send_request;

#[path = "daemon_socket/without_agent_config.rs"]
mod without_agent_config;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_direct_starts_local_daemon_socket_and_remote_agent() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    env.set(
        "XDG_RUNTIME_DIR",
        temp.path().join("runtime").display().to_string(),
    );
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
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

    let socket_path = takd::default_socket_path();
    let config_for_task = config_root.clone();
    let state_for_task = state_root.clone();
    let mut server =
        tokio::spawn(async move { serve_agent(&config_for_task, &state_for_task).await });
    let token_state_root = state_root.clone();
    let token_task = tokio::task::spawn_blocking(move || read_token_wait(&token_state_root, 5));
    let token = tokio::select! {
        token = token_task => token.expect("join wait token").expect("wait token"),
        result = &mut server => panic!("serve_agent stopped before token was ready: {result:?}"),
    };
    let payload = decode_remote_token(&token).expect("decode direct token");
    let node = payload.node.expect("node info");
    let socket_addr = node
        .base_url
        .strip_prefix("http://")
        .expect("direct base url");
    let fetched = fetch_node_info(socket_addr, socket_addr, &payload.bearer_token).await;
    assert_eq!(fetched.node_id, "builder-direct");

    let status = send_request(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "daemon-status".into(),
        }),
    )
    .await;
    assert!(
        matches!(status, Response::StatusSnapshot { .. }),
        "expected local daemon status, got {status:?}"
    );

    server.abort();
}
