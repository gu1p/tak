#![allow(dead_code)]

use std::path::Path;
use std::process::Command as StdCommand;

use super::live_direct::LiveDirectRoots;
use super::tor_smoke::assert_success_with_log;

pub fn wait_for_token(takd: &Path, roots: &LiveDirectRoots) -> String {
    let output = StdCommand::new(takd)
        .args([
            "token",
            "show",
            "--state-root",
            &roots.server_state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "60",
        ])
        .output()
        .expect("run takd token show --wait for direct transport");
    assert_success_with_log(&output, "takd token show --wait", &roots.service_log_path());
    String::from_utf8(output.stdout)
        .expect("token stdout utf8")
        .trim()
        .to_string()
}
