use crate::support;

use std::process::Command as StdCommand;
use support::remote_status::{spawn_status_server, write_inventory};

#[test]
fn status_reports_local_and_remote_sections_without_configured_remotes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");

    let output = StdCommand::new(support::tak_bin())
        .arg("status")
        .env("XDG_CONFIG_HOME", &config_root)
        .env("XDG_STATE_HOME", &state_root)
        .env("TAKD_SOCKET", temp.path().join("missing-takd.sock"))
        .output()
        .expect("run tak status");

    assert!(
        output.status.success(),
        "tak status should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Local"), "missing local section:\n{stdout}");
    assert!(
        stdout.contains("Remote Nodes"),
        "missing remote section:\n{stdout}"
    );
    assert!(
        stdout.contains("daemon=unavailable"),
        "missing local daemon warning:\n{stdout}"
    );
}

#[test]
fn status_prefixes_all_remote_sections_when_remotes_are_configured() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let (base_url, server) = spawn_status_server(true);
    write_inventory(&config_root, "builder-a", &base_url);

    let output = StdCommand::new(support::tak_bin())
        .args(["status", "--node", "builder-a"])
        .env("XDG_CONFIG_HOME", &config_root)
        .env("XDG_STATE_HOME", &state_root)
        .env("TAKD_SOCKET", temp.path().join("missing-takd.sock"))
        .output()
        .expect("run tak status");

    assert!(
        output.status.success(),
        "tak status should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(line_count(&stdout, "Containers"), 1, "stdout:\n{stdout}");
    assert_eq!(line_count(&stdout, "Active Jobs"), 1, "stdout:\n{stdout}");
    assert_eq!(line_count(&stdout, "Remote Nodes"), 1, "stdout:\n{stdout}");
    assert_eq!(
        line_count(&stdout, "Remote Containers"),
        1,
        "stdout:\n{stdout}"
    );
    assert_eq!(
        line_count(&stdout, "Remote Active Jobs"),
        1,
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("//apps/web:build"),
        "missing remote active job:\n{stdout}"
    );
    server.join().expect("status server should exit");
}

fn line_count(output: &str, line: &str) -> usize {
    output
        .lines()
        .filter(|candidate| *candidate == line)
        .count()
}
