use crate::support;

use std::fs;
use std::net::TcpListener;
use std::process::Stdio;

use support::daemon_command_paths::DaemonCommandPaths;
use support::env::env_lock;

#[test]
fn serve_with_tor_test_bind_override_persists_hidden_service_base_url_and_token() {
    let _env_lock = env_lock();
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let paths = DaemonCommandPaths::new(&config_root, &state_root);
    let bind_addr = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").to_string()
    };

    let init = paths
        .rooted_command(&support::takd_bin(), "init")
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let mut child = paths
        .rooted_command(&support::takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .env("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = paths
        .state_command(&support::takd_bin(), &["token", "show"])
        .args(["--wait", "--timeout-secs", "5"])
        .output()
        .expect("run token show");
    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(show.status.success(), "token show should succeed");
    let config = fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(config.contains(".onion"), "missing onion url:\n{config}");
}
