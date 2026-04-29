use crate::support;

use std::process::Command as StdCommand;

#[test]
fn diagnose_tor_is_not_a_takd_command() {
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
            "builder-a",
        ])
        .output()
        .expect("run takd init");
    assert!(init.status.success(), "takd init should succeed");

    let output = StdCommand::new(support::takd_bin())
        .args([
            "diagnose",
            "tor",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .output()
        .expect("run takd diagnose tor");

    assert!(!output.status.success(), "diagnose must not be accepted");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("diagnose"), "unexpected stderr:\n{stderr}");
    assert!(stderr.contains("Usage:"), "missing clap usage:\n{stderr}");
}
