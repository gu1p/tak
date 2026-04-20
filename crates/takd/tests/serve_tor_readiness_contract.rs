use crate::support;

use std::fs;
use std::net::TcpListener;
use std::process::{Command as StdCommand, Stdio};

use support::env::env_lock;

#[test]
fn serve_with_tor_test_bind_override_persists_hidden_service_base_url_and_token() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let bind_addr = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").to_string()
    };

    let init = StdCommand::new(support::takd_bin())
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = StdCommand::new(support::takd_bin())
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "5",
        ])
        .output()
        .expect("run token show");
    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(show.status.success(), "token show should succeed");
    let config = fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(config.contains(".onion"), "missing onion url:\n{config}");
}
