use std::process::Command as StdCommand;

use crate::support;

use super::support_daemon::spawn_peer_daemon_with_state;

#[test]
fn remote_status_reports_failing_daemon_peer_state_and_exits_nonzero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let daemon =
        spawn_peer_daemon_with_state(&socket_path, 1, "unreachable", Some("connect timed out"));

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .output()
        .expect("run tak remote status");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("builder-a transport=tor state=unreachable"));
    assert!(stdout.contains("status=unreachable"));
    daemon.join().expect("peer daemon exits");
}

#[test]
fn status_reports_protocol_mismatch_daemon_peer_state_and_exits_nonzero() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let daemon = spawn_peer_daemon_with_state(
        &socket_path,
        2,
        "protocol_mismatch",
        Some("unsupported ping"),
    );

    let output = StdCommand::new(support::tak_bin())
        .arg("status")
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .output()
        .expect("run tak status");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("builder-a transport=tor state=protocol_mismatch"));
    assert!(stdout.contains("status=protocol_mismatch"));
    daemon.join().expect("peer daemon exits");
}
