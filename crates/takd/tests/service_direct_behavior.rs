#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_proto::decode_remote_token;
use takd::agent::{InitAgentOptions, init_agent, read_token_wait};
use takd::serve_agent;

use crate::support;

use support::env::env_lock;
use support::http::{fetch_node_info, fetch_node_status};

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_direct_persists_ready_base_url_and_serves_node_info() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let pools = vec!["build".to_string()];
    let tags = vec!["builder".to_string()];
    let capabilities = vec!["linux".to_string()];
    init_agent(
        &config_root,
        &state_root,
        InitAgentOptions {
            node_id: Some("builder-direct"),
            display_name: None,
            transport: Some("direct"),
            base_url: Some("http://127.0.0.1:0"),
            pools: &pools,
            tags: &tags,
            capabilities: &capabilities,
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init direct agent");

    let config_for_task = config_root.clone();
    let state_for_task = state_root.clone();
    let server = tokio::spawn(async move {
        let _ = serve_agent(&config_for_task, &state_for_task).await;
    });
    let token_state_root = state_root.clone();
    let token = tokio::task::spawn_blocking(move || read_token_wait(&token_state_root, 5))
        .await
        .expect("join wait token")
        .expect("wait token");
    let payload = decode_remote_token(&token).expect("decode direct token");
    let node = payload.node.expect("node info");
    let socket_addr = node
        .base_url
        .strip_prefix("http://")
        .expect("direct base url");
    let fetched = fetch_node_info(socket_addr, socket_addr, &payload.bearer_token).await;
    let status = fetch_node_status(socket_addr, socket_addr, &payload.bearer_token).await;

    assert_eq!(fetched.node_id, "builder-direct");
    assert_eq!(status.node.expect("status node").node_id, "builder-direct");
    assert!(status.active_jobs.is_empty(), "new agent should be idle");
    assert_eq!(node.transport, "direct");
    assert!(
        node.base_url.starts_with("http://127.0.0.1:") && node.base_url != "http://127.0.0.1:0"
    );
    assert!(
        fs::read_to_string(config_root.join("agent.toml"))
            .expect("read agent config")
            .contains(&node.base_url)
    );
    server.abort();
}
