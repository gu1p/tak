#![allow(dead_code)]

use std::path::Path;
use std::process::{Child, Command as StdCommand, Output};

use super::run::tak_bin;
pub use super::takd_binary::{resolve_takd_bin, resolve_takd_bin_with_bootstrap, takd_bin};
use super::tor_probe_env::apply_live_tor_probe_env;

pub struct ChildGuard {
    pub child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn tak_command(workspace_root: &Path, config_root: &Path) -> StdCommand {
    let mut command = StdCommand::new(tak_bin());
    let state_root = config_root
        .parent()
        .expect("client config root should have a parent")
        .join("client-state");
    apply_live_tor_probe_env(
        command
            .current_dir(workspace_root)
            .env("XDG_CONFIG_HOME", config_root)
            .env("XDG_STATE_HOME", state_root)
            .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock")),
    );
    command
}

pub fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn assert_success_with_log(output: &Output, command: &str, log_path: &Path) {
    let log = std::fs::read_to_string(log_path)
        .unwrap_or_else(|_| format!("missing service log at {}", log_path.display()));
    assert!(
        output.status.success(),
        "{command} should succeed\nstdout:\n{}\nstderr:\n{}\nservice.log:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        log
    );
}
