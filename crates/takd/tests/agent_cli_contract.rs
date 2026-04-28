//! Contract tests for `takd` agent lifecycle commands.

use crate::support;

use std::fs;
use std::process::Command as StdCommand;

#[path = "agent_cli_contract/image_cache.rs"]
mod image_cache;

#[test]
fn init_persists_pending_tor_agent_and_token_show_requires_readiness() {
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
    assert!(
        init.status.success(),
        "takd init should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    let config = fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(
        config.contains("transport = \"tor\""),
        "unexpected config: {config}"
    );
    assert!(
        !state_root.join("agent.token").exists(),
        "init should not persist a token before hidden-service readiness"
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
        "takd token show should fail before hidden-service readiness"
    );
    assert!(
        String::from_utf8_lossy(&show.stderr).contains("not ready"),
        "unexpected stderr:\n{}",
        String::from_utf8_lossy(&show.stderr)
    );
}
