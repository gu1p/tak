#![cfg(unix)]

use std::process::Command;

use crate::support;
use support::remote_daemon_v2::{FakeRemoteDaemon, remote};

#[test]
fn status_renders_daemon_owned_remote_failure_and_exits_nonzero() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![
            serde_json::json!({"type": "DaemonStatus",
            "status": {"active_leases": 0, "pending_requests": 0, "limiter_count": 0}}),
            serde_json::json!({"type": "RemoteStatus", "remotes": [{
                "remote": remote("builder-z"), "snapshot": null, "detail_base64": null,
                "error": "worker protocol mismatch; upgrade tak, takd, and workers together",
                "peer": null
            }]}),
        ],
    );

    let output = Command::new(support::tak_bin())
        .args(["status", "--node", "builder-z"])
        .env("TAKD_SOCKET", daemon.socket())
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output()
        .expect("run tak status");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Remote Nodes") && stdout.contains("builder-z"),
        "{stdout}"
    );
    assert!(
        stdout.contains("upgrade tak, takd, and workers together"),
        "{stdout}"
    );
    let requests = daemon.finish();
    assert_eq!(requests[0]["protocol_version"], 2);
    assert_eq!(requests[0]["operation"]["type"], "GetDaemonStatus");
    assert_eq!(requests[1]["operation"]["type"], "GetRemoteStatus");
}
