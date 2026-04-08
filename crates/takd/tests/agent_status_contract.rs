use std::process::Command as StdCommand;

#[test]
fn status_reports_pending_tor_readiness_after_init() {
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
            "builder-a",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

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
    assert!(status.status.success(), "takd status should succeed");
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("transport: tor"),
        "missing transport:\n{stdout}"
    );
    assert!(
        stdout.contains("readiness: pending"),
        "missing readiness:\n{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "log_path: {}",
            state_root.join("service.log").display()
        )),
        "missing log path:\n{stdout}"
    );
    assert!(
        stdout.contains("log_state: missing"),
        "missing log state:\n{stdout}"
    );
}
