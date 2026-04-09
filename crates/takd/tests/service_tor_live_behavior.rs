#![allow(clippy::await_holding_lock)]

mod support;

use std::fs;

use tak_proto::decode_remote_token;

use support::env::env_lock;
use support::live_tor_cli::{LiveTorRoots, init_tor_agent, spawn_tor_agent, wait_for_token};
use support::live_tor_http::wait_for_onion_node_info;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_real_tor_publishes_reachable_onion_url() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let roots = LiveTorRoots::new(temp.path());

    init_tor_agent(&roots, "builder-tor-live");
    let _child = spawn_tor_agent(&roots);

    let token = wait_for_token(&roots);
    let payload = decode_remote_token(&token).expect("decode tor token");
    let node = payload.node.expect("token node");
    let fetched =
        wait_for_onion_node_info(temp.path(), &node.base_url, &payload.bearer_token).await;

    assert_eq!(node.node_id, "builder-tor-live");
    assert_eq!(node.transport, "tor");
    assert!(node.base_url.starts_with("http://"));
    assert!(node.base_url.contains(".onion"));
    assert_eq!(fetched.node_id, node.node_id);
    assert_eq!(fetched.base_url, node.base_url);
    assert_eq!(fetched.transport, "tor");
    assert!(
        fs::read_to_string(roots.config_root.join("agent.toml"))
            .expect("read agent config")
            .contains(&node.base_url)
    );
}
