#![allow(clippy::await_holding_lock)]

use std::net::TcpListener as StdTcpListener;

use tak_proto::decode_tor_invite;
use takd::agent::{InitAgentOptions, init_agent, read_token_wait};
use takd::serve_agent;

use crate::support;

use support::env::{EnvGuard, env_lock};
use support::http::fetch_node_info;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_simulated_tor_retries_initial_startup_failures_until_token_is_ready() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let bind_addr = {
        let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind free port");
        let addr = listener.local_addr().expect("free addr").to_string();
        drop(listener);
        addr
    };
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", bind_addr.clone());
    env.set("TAKD_TEST_TOR_FAIL_STARTUP_ONCE", "1");
    env.set("TAKD_TOR_RECOVERY_INITIAL_BACKOFF_MS", "50");
    env.set("TAKD_TOR_RECOVERY_MAX_BACKOFF_MS", "50");

    let empty = Vec::<String>::new();
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    init_agent(
        &config_root,
        &state_root,
        InitAgentOptions {
            node_id: Some("builder-tor"),
            display_name: None,
            transport: Some("tor"),
            base_url: None,
            pools: &empty,
            tags: &empty,
            capabilities: &empty,
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init tor agent");

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
    let base_url = decode_tor_invite(&token).expect("decode tor invite");
    let node = fetch_node_info(&bind_addr, "builder-tor.onion", "").await;

    assert_eq!(node.node_id, "builder-tor");
    assert_eq!(node.base_url, base_url);
    server.abort();
}
