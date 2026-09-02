#![cfg(unix)]

use std::process::Command;

use crate::support;
use support::remote_daemon_v2::FakeRemoteDaemon;

#[test]
fn remote_remove_preserves_output_via_daemon_v2_inventory() {
    let root = tempfile::tempdir().expect("temp root");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteRemoved", "node_id": "builder-a", "removed": true
        })],
    );

    let output = Command::new(support::tak_bin())
        .args(["remote", "remove", "builder-a"])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run remote remove");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "removed remote builder-a\n"
    );
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "RemoveRemote");
    assert_eq!(requests[0]["operation"]["node_id"], "builder-a");
}
