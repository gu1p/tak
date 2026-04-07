#![allow(clippy::await_holding_lock)]

use takd::run_daemon;

mod support;

use support::env::{EnvGuard, env_lock};
use support::http::wait_for_node_info;
use support::protocol::free_bind_addr;

#[tokio::test(flavor = "multi_thread")]
async fn run_daemon_spawns_optional_tor_hidden_service_from_env() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let bind_addr = free_bind_addr();
    env.set(
        "TAKD_DB_PATH",
        temp.path().join("state/takd.sqlite").display().to_string(),
    );
    env.set("TAKD_TOR_HS_NICKNAME", "daemon-tor");
    env.set(
        "TAKD_TOR_STATE_DIR",
        temp.path().join("arti/state").display().to_string(),
    );
    env.set(
        "TAKD_TOR_CACHE_DIR",
        temp.path().join("arti/cache").display().to_string(),
    );
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", bind_addr.clone());
    env.set("TAKD_NODE_ID", "daemon-tor");
    env.set("TAKD_DISPLAY_NAME", "daemon-tor");
    env.set("TAKD_BEARER_TOKEN", "secret");
    env.set("TAKD_NODE_TRANSPORT", "tor");

    let socket_path = temp.path().join("run/takd.sock");
    let daemon = tokio::spawn(async move { run_daemon(&socket_path).await });
    let node = wait_for_node_info(&bind_addr, "daemon-tor.onion", "secret").await;

    assert_eq!(node.node_id, "daemon-tor");
    assert_eq!(node.transport, "tor");
    assert_eq!(node.base_url, "http://daemon-tor.onion");
    daemon.abort();
}
