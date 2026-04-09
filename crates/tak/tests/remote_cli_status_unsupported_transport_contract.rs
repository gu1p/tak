mod support;

use std::process::Command as StdCommand;

use support::remote_status::write_inventory_entries;

#[test]
fn remote_status_reports_unsupported_transport_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    write_inventory_entries(
        &config_root,
        &[("builder-weird", "http://127.0.0.1:9", "udp", true)],
    );

    let output = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"))
        .args(["remote", "status", "--node", "builder-weird"])
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote status");
    assert!(
        !output.status.success(),
        "unsupported transport should fail remote status"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unsupported remote transport `udp`"),
        "missing unsupported transport error:\n{stdout}"
    );
}
