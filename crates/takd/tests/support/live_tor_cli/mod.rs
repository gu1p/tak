#![allow(dead_code)]

mod command_assert;

use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Stdio};

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

    pub fn service_log_path(&self) -> PathBuf {
        self.state_root.join("service.log")
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
    let output = StdCommand::new(takd_bin())
        .args([
            "init",
            "--config-root",
            &roots.config_root.display().to_string(),
            "--state-root",
            &roots.state_root.display().to_string(),
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
    let child = StdCommand::new(takd_bin())
        .args([
            "serve",
            "--config-root",
            &roots.config_root.display().to_string(),
            "--state-root",
            &roots.state_root.display().to_string(),
        ])
        .env("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1000")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");
    ChildGuard { child }
}

pub fn wait_for_token(roots: &LiveTorRoots) -> String {
    let output = StdCommand::new(takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &roots.state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "360",
        ])
        .output()
        .expect("run takd token show --wait");
    assert_success_with_log(&output, "takd token show --wait", &roots.service_log_path());
    String::from_utf8(output.stdout)
        .expect("token stdout utf8")
        .trim()
        .to_string()
}
