#![allow(dead_code)]

use std::path::Path;

use super::live_direct::LiveDirectRoots;
use super::tor_smoke::{assert_success_with_log, tak_command};

pub fn add_remote(workspace_root: &Path, roots: &LiveDirectRoots, token: &str) {
    let output = tak_command(workspace_root, &roots.client_config_root)
        .args(["remote", "add", token])
        .output()
        .expect("run tak remote add for direct transport");
    assert_success_with_log(&output, "tak remote add", &roots.service_log_path());
}
