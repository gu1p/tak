#![allow(clippy::await_holding_lock)]

use std::net::TcpListener as StdTcpListener;

use takd::{SubmitAttemptStore, TorHiddenServiceRuntimeConfig, run_remote_v1_tor_hidden_service};

mod support;

use support::env::{EnvGuard, env_lock};
use support::http::wait_for_node_info;

#[tokio::test]
async fn tor_hidden_service_runtime_uses_test_bind_addr_and_serves_node_info() {
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
    env.set("TAKD_NODE_ID", "builder-hs");
    env.set("TAKD_DISPLAY_NAME", "builder-hs");
    env.set("TAKD_BEARER_TOKEN", "secret");
    env.set("TAKD_NODE_TRANSPORT", "tor");

    let store =
        SubmitAttemptStore::with_db_path(temp.path().join("remote.sqlite")).expect("submit store");
    let server = tokio::spawn(run_remote_v1_tor_hidden_service(
        TorHiddenServiceRuntimeConfig {
            nickname: "builder-hs".to_string(),
            state_dir: temp.path().join("arti/state"),
            cache_dir: temp.path().join("arti/cache"),
        },
        store,
    ));

    let node = wait_for_node_info(&bind_addr, "builder-hs.onion", "secret").await;
    assert_eq!(node.node_id, "builder-hs");
    assert_eq!(node.transport, "tor");
    assert_eq!(node.base_url, "http://builder-hs.onion");

    server.abort();
}
