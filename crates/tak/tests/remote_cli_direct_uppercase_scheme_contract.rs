mod support;

use std::fs;
use std::net::TcpListener;

use support::live_direct::{LiveDirectRoots, init_direct_agent_with_base_url, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;

#[test]
fn remote_add_succeeds_when_direct_agent_started_from_uppercase_http_base_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace root");

    let roots = LiveDirectRoots::new(temp.path());
    let base_url = format!("HTTP://127.0.0.1:{}", reserved_port());
    let takd = takd_bin();
    init_direct_agent_with_base_url(&takd, &roots, "builder-uppercase", &base_url);

    let _agent = spawn_direct_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);
}

fn reserved_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("listener addr").port()
}
