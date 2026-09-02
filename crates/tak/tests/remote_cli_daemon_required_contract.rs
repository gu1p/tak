use std::process::Command;

use crate::support;

#[test]
fn remote_inventory_commands_require_local_takd_without_config_fallback() {
    let root = tempfile::tempdir().expect("temp root");
    let config = root.path().join("config");
    let inventory = config.join("tak/remotes.toml");
    std::fs::create_dir_all(inventory.parent().expect("inventory parent")).expect("config dir");
    std::fs::write(
        &inventory,
        "version = 1\n[[remotes]]\nnode_id = \"client-side-secret\"\nbase_url = \"http://127.0.0.1:9\"\nbearer_token = \"must-not-be-read\"\ntransport = \"direct\"\n",
    )
    .expect("legacy inventory");

    let output = Command::new(support::tak_bin())
        .args(["remote", "list"])
        .env("TAKD_SOCKET", root.path().join("missing.sock"))
        .env("XDG_CONFIG_HOME", &config)
        .output()
        .expect("run remote list");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("start `takd serve`"), "{stderr}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("client-side-secret"));
}
