#![allow(dead_code)]
mod command_assert;

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};

use self::command_assert::{assert_success, assert_success_with_log};
use super::cli::takd_bin;

pub struct LiveTorRoots {
    pub config_root: PathBuf,
    pub state_root: PathBuf,
}

impl LiveTorRoots {
    pub fn new(base: &Path) -> Self {
        Self {
            config_root: base.join("config"),
            state_root: base.join("state"),
        }
    }
}

pub struct ChildGuard {
    child: Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn init_tor_agent(roots: &LiveTorRoots, node_id: &str) {
    let paths =
        super::daemon_command_paths::DaemonCommandPaths::new(&roots.config_root, &roots.state_root);
    let output = paths
        .rooted_command(&takd_bin(), "init")
        .args([
            "--node-id",
            node_id,
            "--pool",
            "build",
            "--tag",
            "builder",
            "--capability",
            "linux",
        ])
        .output()
        .expect("run takd init");
    assert_success(&output, "takd init");
}

pub fn spawn_tor_agent(roots: &LiveTorRoots) -> ChildGuard {
    let paths =
        super::daemon_command_paths::DaemonCommandPaths::new(&roots.config_root, &roots.state_root);
    let child = paths
        .rooted_command(&takd_bin(), "serve")
        .env("XDG_RUNTIME_DIR", paths.runtime_root())
        .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
        .env("MOCK_CONTAINER", "true")
        .env("TAK_TEST_IGNORE_HOST_USAGE", "true")
        .env("TAKD_HOST_BASELINE_SAMPLE_MS", "0")
        .env("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1000")
        .env("TAKD_TOR_RECOVERY_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_RECOVERY_PROBE_BACKOFF_MS", "1000")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");
    ChildGuard { child }
}

pub fn wait_for_token(roots: &LiveTorRoots) -> String {
    let paths =
        super::daemon_command_paths::DaemonCommandPaths::new(&roots.config_root, &roots.state_root);
    let output = paths
        .state_command(&takd_bin(), &["token", "show"])
        .args(["--wait", "--timeout-secs", "360"])
        .output()
        .expect("run takd token show --wait");
    let log_path = roots.state_root.join("service.log");
    assert_success_with_log(&output, "takd token show --wait", &log_path);
    String::from_utf8(output.stdout)
        .expect("token stdout utf8")
        .trim()
        .to_string()
}
