#![allow(dead_code)]

use std::path::Path;

use super::live_tor::LiveTorRoots;
use super::tor_smoke::{assert_success_with_log, tak_command};

pub fn add_remote(workspace_root: &Path, roots: &LiveTorRoots, token: &str) {
    let output = tak_command(workspace_root, &roots.client_config_root)
        .args(["remote", "add", token])
        .output()
        .expect("run tak remote add");
    assert_success_with_log(
        &output,
        "tak remote add",
        &roots.server_state_root.join("service.log"),
    );
}

pub fn assert_remote_list(workspace_root: &Path, roots: &LiveTorRoots, node_id: &str) {
    let output = tak_command(workspace_root, &roots.client_config_root)
        .args(["remote", "list"])
        .output()
        .expect("run tak remote list");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "tak remote list should succeed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(node_id),
        "missing live tor node id in list:\n{stdout}"
    );
    assert!(
        stdout.contains("tor"),
        "missing tor transport in list:\n{stdout}"
    );
}
