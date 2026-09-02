#![cfg(unix)]

use std::process::Command;

use crate::support;
use support::remote_daemon_v2::{FakeRemoteDaemon, remote};

#[test]
fn status_preserves_local_and_remote_sections_with_v2_remote_status() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![
            serde_json::json!({
                "type": "DaemonStatus",
                "status": {"active_leases": 0, "pending_requests": 0, "limiter_count": 0}
            }),
            serde_json::json!({
                "type": "RemoteStatus",
                "remotes": [{
                    "remote": remote("builder-a"),
                    "snapshot": {"protocol_version": 2, "node_id": "builder-a", "healthy": true,
                        "sampled_at_ms": 1, "capacity": {"cpu_millis": 8000, "memory_bytes": 16000,
                        "execution_slots": 8}, "usage": {"cpu_millis": 1000, "memory_bytes": 4000,
                        "execution_slots": 2}, "queue_depth": 1, "cached_content": [], "processes": []},
                    "detail_base64": null, "error": null, "peer": null
                }]
            }),
        ],
    );

    let output = Command::new(support::tak_bin())
        .args(["status", "--node", "builder-a"])
        .env("TAKD_SOCKET", daemon.socket())
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output()
        .expect("run tak status");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Local") && stdout.contains("Remote Nodes"),
        "{stdout}"
    );
    assert!(
        stdout.contains("builder-a transport=tor state=ready"),
        "{stdout}"
    );
    let requests = daemon.finish();
    assert_eq!(requests[0]["protocol_version"], 2);
    assert_eq!(requests[0]["operation"]["type"], "GetDaemonStatus");
    assert_eq!(requests[1]["operation"]["type"], "GetRemoteStatus");
    assert_eq!(
        requests[1]["operation"]["node_ids"],
        serde_json::json!(["builder-a"])
    );
}

#[test]
fn status_requires_local_daemon_for_remote_status_without_inventory_fallback() {
    let root = tempfile::tempdir().expect("temp root");
    let output = Command::new(support::tak_bin())
        .arg("status")
        .env("TAKD_SOCKET", root.path().join("missing.sock"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output()
        .expect("run tak status");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("start `takd serve`"));
}
