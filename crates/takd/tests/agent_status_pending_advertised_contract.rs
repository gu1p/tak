use crate::support;

use std::process::Command as StdCommand;

use takd::agent::{
    TransportHealth, TransportState, persist_ready_base_url, write_transport_health,
};

#[test]
fn pending_advertised_tor_status_is_unverified_and_token_show_is_blocked() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");

    let init = StdCommand::new(support::takd_bin())
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
            "--node-id",
            "builder-pending",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let base_url = "http://builder-pending.onion";
    persist_ready_base_url(&config_root, &state_root, base_url).expect("persist advertised url");
    write_transport_health(
        &state_root,
        &TransportHealth::new(
            TransportState::Pending,
            Some(base_url.to_string()),
            Some(
                "Arti onion-service state=Bootstrapping; Arti bootstrap: 74%: connecting to the Tor network"
                    .to_string(),
            ),
        ),
    )
    .expect("write pending health");

    let status = StdCommand::new(support::takd_bin())
        .args([
            "status",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd status");
    assert!(status.status.success(), "takd status should succeed");
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("readiness: advertised")
            && stdout.contains("transport_state: pending")
            && stdout.contains("reachability: unverified")
            && stdout.contains("base_url: http://builder-pending.onion")
            && stdout.contains("transport_detail: Arti onion-service state=Bootstrapping")
            && stdout.contains("Arti bootstrap: 74%"),
        "missing pending advertised status fields:\n{stdout}"
    );

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd token show");
    assert!(
        !show.status.success(),
        "pending transport must not produce a usable token\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&show.stdout),
        String::from_utf8_lossy(&show.stderr)
    );
    assert!(
        String::from_utf8_lossy(&show.stderr).contains("tor transport is pending"),
        "unexpected token show stderr:\n{}",
        String::from_utf8_lossy(&show.stderr)
    );
}
