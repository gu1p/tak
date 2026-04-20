use crate::support;

use std::net::TcpListener;
use std::process::Command as StdCommand;

use support::remote_cli::remote_inventory_path;
use tak_proto::encode_tor_invite;

#[test]
fn remote_add_reports_exhausted_tor_probe_retry_window_and_keeps_inventory_empty() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind free port");
    let bind_addr = listener.local_addr().expect("listener addr").to_string();
    drop(listener);
    let invite = encode_tor_invite("http://builder-unreachable-hidden-service.onion")
        .expect("encode invite");

    let add = StdCommand::new(support::tak_bin())
        .args(["remote", "add", &invite])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", bind_addr)
        .env("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "40")
        .env("TAK_TEST_TOR_PROBE_BACKOFF_MS", "10")
        .output()
        .expect("run tak remote add");

    let stderr = String::from_utf8_lossy(&add.stderr);
    assert!(!add.status.success(), "tak remote add should fail");
    assert!(
        stderr.contains(
            "failed to probe remote node at http://builder-unreachable-hidden-service.onion via tor"
        ),
        "unexpected stderr:\n{stderr}"
    );
    assert!(stderr.contains("did not become reachable within"));
    assert!(stderr.contains("freshly started takd hidden service may still be propagating"));
    assert!(!remote_inventory_path(&config_root).exists());
}
