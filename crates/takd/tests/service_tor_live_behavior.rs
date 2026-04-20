#![allow(clippy::await_holding_lock)]

use crate::support;

use std::fs;

use tak_proto::decode_tor_invite;

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
    let base_url = decode_tor_invite(&token).expect("decode tor invite");
    let fetched = wait_for_onion_node_info(temp.path(), &base_url, "").await;

    assert_eq!(fetched.node_id, "builder-tor-live");
    assert_eq!(fetched.base_url, base_url);
    assert_eq!(fetched.transport, "tor");
    assert!(
        fs::read_to_string(roots.config_root.join("agent.toml"))
            .expect("read agent config")
            .contains(&base_url)
    );
}
