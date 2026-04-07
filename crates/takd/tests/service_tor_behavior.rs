#![allow(clippy::await_holding_lock)]

use std::fs;
use std::net::TcpListener as StdTcpListener;

use tak_proto::decode_remote_token;
use takd::agent::{InitAgentOptions, init_agent, read_token_wait};
use takd::serve_agent;

mod support;

use support::env::{EnvGuard, env_lock};
use support::http::fetch_node_info;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_tor_uses_test_bind_addr_and_persists_onion_base_url() {
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
        },
    )
    .expect("init tor agent");

    let config_for_task = config_root.clone();
    let state_for_task = state_root.clone();
    let server = tokio::spawn(async move {
        let _ = serve_agent(&config_for_task, &state_for_task).await;
    });
    let payload = decode_remote_token(&read_token_wait(&state_root, 5).expect("wait token"))
        .expect("decode tor token");
    let node = payload.node.expect("node info");
    let fetched = fetch_node_info(&bind_addr, "builder-tor.onion", &payload.bearer_token).await;

    assert_eq!(fetched.node_id, "builder-tor");
    assert_eq!(node.transport, "tor");
    assert!(node.base_url.starts_with("http://") && node.base_url.contains(".onion"));
    assert!(
        fs::read_to_string(config_root.join("agent.toml"))
            .expect("read agent config")
            .contains(".onion")
    );
    server.abort();
}
