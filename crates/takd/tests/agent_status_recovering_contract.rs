use std::process::Command as StdCommand;

use takd::agent::{
    TransportHealth, TransportState, persist_ready_base_url, write_transport_health,
};

#[test]
fn status_reports_recovering_transport_state_when_tor_relaunch_is_in_progress() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");

    let init = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "init",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
            "--node-id",
            "builder-status",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let base_url = "http://builder-status.onion";
    persist_ready_base_url(&config_root, &state_root, base_url).expect("persist ready base url");
    write_transport_health(
        &state_root,
        &TransportHealth::new(
            TransportState::Recovering,
            Some(base_url.to_string()),
            Some("rendezvous accept failures exceeded threshold".to_string()),
        ),
    )
    .expect("write transport health");

    let status = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "status",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd status");

    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status.status.success(), "takd status should succeed");
    assert!(
        stdout.contains("readiness: advertised"),
        "missing readiness:\n{stdout}"
    );
    assert!(
        stdout.contains("transport_state: recovering"),
        "missing transport_state:\n{stdout}"
    );
    assert!(
        stdout.contains("base_url: http://builder-status.onion"),
        "missing base_url:\n{stdout}"
    );
    assert!(
        !stdout.contains("reachability: verified"),
        "status should not claim verified reachability while recovering:\n{stdout}"
    );
}
