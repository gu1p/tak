#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Output};

pub struct ChildGuard {
    pub child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

pub fn takd_bin() -> PathBuf {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_takd") {
        return PathBuf::from(path);
    }

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("target"));
    let binary = target_dir
        .join("debug")
        .join(format!("takd{}", std::env::consts::EXE_SUFFIX));
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = StdCommand::new(cargo)
        .current_dir(workspace_root())
        .args(["build", "-p", "takd", "--bin", "takd"])
        .status()
        .expect("build takd binary for live tor smoke test");
    assert!(status.success(), "building takd binary should succeed");
    assert!(
        binary.is_file(),
        "missing takd binary at {}",
        binary.display()
    );
    binary
}

pub fn tak_command(workspace_root: &Path, config_root: &Path) -> StdCommand {
    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command
        .current_dir(workspace_root)
        .env("XDG_CONFIG_HOME", config_root)
        .env("TAK_TOR_PROBE_TIMEOUT_MS", "300000")
        .env("TAK_TOR_PROBE_BACKOFF_MS", "1000")
        .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock"));
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
