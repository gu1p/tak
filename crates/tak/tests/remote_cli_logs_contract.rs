#![cfg(unix)]

use std::process::Command;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::support;
use support::remote_daemon_v2::FakeRemoteDaemon;

#[test]
fn remote_logs_preserves_output_via_daemon_v2_read() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteRead",
            "node_id": "builder-a",
            "http_status": 200,
            "body_base64": STANDARD.encode(b"booting takd\nremote service ready\n")
        })],
    );

    let output = Command::new(support::tak_bin())
        .args(["remote", "logs", "--node", "builder-a", "--all"])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run remote logs");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "booting takd\nremote service ready\n"
    );
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ReadRemote");
    assert_eq!(requests[0]["operation"]["node_id"], "builder-a");
    assert_eq!(requests[0]["operation"]["path"], "/v2/worker/logs?all=true");
}
