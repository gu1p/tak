use std::process::Command as StdCommand;

use crate::support;
use support::remote_status::write_inventory_entries;

use super::support_daemon::{spawn_direct_status_server, spawn_peer_daemon};

#[test]
fn remote_status_uses_daemon_peer_snapshot_when_reachable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let daemon = spawn_peer_daemon(&socket_path, 1);

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-a"])
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .output()
        .expect("run tak remote status");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Nodes"), "missing nodes section:\n{stdout}");
    assert!(stdout.contains("builder-a transport=tor state=connected"));
    assert!(stdout.contains("jobs=1"), "missing daemon load:\n{stdout}");
    daemon.join().expect("peer daemon exits");
}

#[test]
fn remote_status_combines_daemon_tor_peers_with_direct_inventory_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let config_root = temp.path().join("config");
    let (direct_base_url, direct_server) = spawn_direct_status_server("builder-direct");
    write_inventory_entries(
        &config_root,
        &[
            ("builder-a", "http://builder-a.onion", "tor", true),
            ("builder-direct", &direct_base_url, "direct", true),
        ],
    );
    let daemon = spawn_peer_daemon(&socket_path, 1);

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status"])
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("builder-a transport=tor state=connected"));
    assert!(stdout.contains("builder-direct transport=direct state=ready"));
    direct_server.join().expect("direct status server exits");
    daemon.join().expect("peer daemon exits");
}

#[test]
fn status_uses_daemon_peer_snapshot_for_remote_section_when_reachable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let daemon = spawn_peer_daemon(&socket_path, 2);

    let output = StdCommand::new(support::tak_bin())
        .arg("status")
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .output()
        .expect("run tak status");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local"), "missing local section:\n{stdout}");
    assert!(stdout.contains("local status=ok daemon=ok"));
    assert!(
        stdout.contains("Remote Nodes"),
        "missing remote nodes section:\n{stdout}"
    );
    assert!(stdout.contains("builder-a transport=tor state=connected"));
    daemon.join().expect("peer daemon exits");
}

// Regression: a running takd that simply has no live session for a configured
// Tor remote used to fall through to the direct-probe stub and report the
// misleading "Tor remote status requires local takd serve" — even though takd
// was serving. It must now surface an honest "not reported by local takd" status.
#[test]
fn remote_status_marks_configured_tor_peer_the_daemon_does_not_report() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    let config_root = temp.path().join("config");
    // Inventory has builder-z, but the (reachable) daemon only reports builder-a.
    write_inventory_entries(
        &config_root,
        &[("builder-z", "http://builder-z.onion", "tor", true)],
    );
    let daemon = spawn_peer_daemon(&socket_path, 1);

    let output = StdCommand::new(support::tak_bin())
        .args(["remote", "status", "--node", "builder-z"])
        .env("TAKD_SOCKET", &socket_path)
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");

    // The node genuinely has no status, so the command still exits non-zero...
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ...but with an honest message, not the old takd-is-down claim.
    assert!(
        stdout.contains("builder-z transport=tor"),
        "missing builder-z row:\n{stdout}"
    );
    assert!(
        stdout.contains("not reported by local takd peer manager"),
        "missing honest not-reported status:\n{stdout}"
    );
    assert!(
        !stdout.contains("requires local takd serve"),
        "still emits the misleading stub message:\n{stdout}"
    );
    daemon.join().expect("peer daemon exits");
}
