#![cfg(unix)]

use std::process::Command;

use crate::support;
use support::remote_daemon_v2::{FakeRemoteDaemon, healthy_status};

#[test]
fn remote_status_watch_refreshes_through_daemon_v2_until_test_limit() {
    let root = tempfile::tempdir().expect("temp root");
    let response = || {
        serde_json::json!({
            "type": "RemoteStatus", "remotes": [healthy_status("builder-a")]
        })
    };
    let daemon = FakeRemoteDaemon::spawn(root.path(), vec![response(), response()]);

    let output = Command::new(support::tak_bin())
        .args(["remote", "status", "--watch", "--interval-ms", "1"])
        .env("TAKD_SOCKET", daemon.socket())
        .env("TAK_TEST_REMOTE_STATUS_MAX_POLLS", "2")
        .output()
        .expect("run remote status watch");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .matches("Nodes")
            .count()
            >= 2
    );
    let requests = daemon.finish();
    assert!(
        requests
            .iter()
            .all(|request| request["operation"]["type"] == "GetRemoteStatus")
    );
    assert!(
        requests
            .iter()
            .all(|request| request["operation"]["node_ids"] == serde_json::json!([]))
    );
}
