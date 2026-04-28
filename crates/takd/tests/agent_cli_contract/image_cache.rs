use std::fs;
use std::process::Command as StdCommand;

use crate::support;

#[test]
fn init_persists_image_cache_budget_when_configured_in_gb() {
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
            "builder-cache",
            "--image-cache-budget-gb",
            "50",
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
        config.contains("[image_cache]"),
        "missing image_cache table: {config}"
    );
    assert!(
        config.contains("budget_gb = 50.0"),
        "missing configured budget: {config}"
    );
    assert!(
        config.contains("mutable_tag_ttl_secs = 86400"),
        "missing mutable tag TTL: {config}"
    );
    assert!(
        config.contains("low_disk_min_free_percent = 10.0"),
        "missing free-space percent floor: {config}"
    );
    assert!(
        config.contains("low_disk_min_free_gb = 10.0"),
        "missing free-space GB floor: {config}"
    );
}
