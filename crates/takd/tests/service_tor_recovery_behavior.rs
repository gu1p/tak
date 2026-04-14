#![allow(clippy::await_holding_lock)]

use std::fs;
use std::net::TcpListener as StdTcpListener;

use tak_proto::decode_remote_token;
use takd::agent::{InitAgentOptions, TransportState, init_agent, read_token_wait};
use takd::serve_agent;

mod support;

use support::env::{EnvGuard, env_lock};
use support::http::fetch_node_info;
use support::transport_health::wait_for_transport_state;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_simulated_tor_relaunches_in_process_and_keeps_the_same_onion_base_url() {
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
    env.set("TAKD_TEST_TOR_FORCE_RECOVERY_AFTER_MS", "150");
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
    let payload = decode_remote_token(&token).expect("decode tor token");
    let first = fetch_node_info(&bind_addr, "builder-tor.onion", &payload.bearer_token).await;

    let recovering = wait_for_transport_state(&state_root, TransportState::Recovering).await;
    assert_eq!(
        recovering.base_url.as_deref(),
        Some("http://builder-tor.onion")
    );

    let ready = wait_for_transport_state(&state_root, TransportState::Ready).await;
    assert_eq!(ready.base_url.as_deref(), Some("http://builder-tor.onion"));

    let second = fetch_node_info(&bind_addr, "builder-tor.onion", &payload.bearer_token).await;

    assert_eq!(first.node_id, "builder-tor");
    assert_eq!(first.base_url, "http://builder-tor.onion");
    assert_eq!(second.node_id, first.node_id);
    assert_eq!(second.base_url, first.base_url);
    assert!(
        fs::read_to_string(config_root.join("agent.toml"))
            .expect("read agent config")
            .contains("http://builder-tor.onion")
    );
    server.abort();
}
