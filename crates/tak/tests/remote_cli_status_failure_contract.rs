#![cfg(unix)]

use std::process::Command;

use crate::support;
use support::remote_daemon_v2::{FakeRemoteDaemon, healthy_status, remote};

#[test]
fn remote_status_renders_daemon_partial_failures_in_node_order() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteStatus", "remotes": [
                healthy_status("builder-a"),
                {"remote": remote("builder-z"), "snapshot": null, "detail_base64": null,
                 "error": "worker unavailable", "peer": null}
            ]
        })],
    );

    let output = Command::new(support::tak_bin())
        .args(["remote", "status"])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run remote status");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("builder-a") && stdout.contains("status=ok"),
        "{stdout}"
    );
    assert!(
        stdout.contains("builder-z") && stdout.contains("status=worker unavailable"),
        "{stdout}"
    );
    assert!(stdout.find("builder-a") < stdout.find("builder-z"));
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "GetRemoteStatus");
}
