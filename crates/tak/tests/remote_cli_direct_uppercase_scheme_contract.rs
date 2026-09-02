use crate::support;

use std::fs;
use std::net::TcpListener;
use std::path::Path;

use support::live_direct::{LiveDirectRoots, init_direct_agent_with_base_url, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;

#[test]
fn remote_add_succeeds_when_direct_agent_started_from_uppercase_http_base_url() {
    fs::create_dir_all(".tmp").expect("create test temporary root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace root");

    let roots = LiveDirectRoots::new(temp.path());
    let base_url = format!("HTTP://127.0.0.1:{}", reserved_port());
    let takd = takd_bin();
    init_direct_agent_with_base_url(&takd, &roots, "builder-uppercase", &base_url);

    let _agent = spawn_direct_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    let (_daemon, env) = support::v2_remote_daemon::spawn(temp.path(), &workspace_root);
    let daemon_socket = env.get("TAKD_SOCKET").expect("local daemon socket");
    add_remote(&workspace_root, &roots, &token, Path::new(daemon_socket));
}

fn reserved_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("listener addr").port()
}
