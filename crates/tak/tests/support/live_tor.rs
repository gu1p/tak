#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

use super::tor_smoke::{ChildGuard, assert_success, assert_success_with_log, tak_command};

pub struct LiveTorRoots {
    pub server_config_root: PathBuf,
    pub server_state_root: PathBuf,
    pub client_config_root: PathBuf,
}

impl LiveTorRoots {
    pub fn new(base: &Path) -> Self {
        Self {
            server_config_root: base.join("server-config"),
            server_state_root: base.join("server-state"),
            client_config_root: base.join("client-config"),
        }
    }

    pub fn service_log_path(&self) -> PathBuf {
        self.server_state_root.join("service.log")
    }
}

pub fn init_tor_agent(takd: &Path, roots: &LiveTorRoots, node_id: &str) {
    let output = StdCommand::new(takd)
        .args([
            "init",
            "--config-root",
            &roots.server_config_root.display().to_string(),
            "--state-root",
            &roots.server_state_root.display().to_string(),
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

pub fn spawn_tor_agent(takd: &Path, roots: &LiveTorRoots) -> ChildGuard {
    spawn_tor_agent_with_env(takd, roots, &[])
}

pub fn spawn_tor_agent_with_env(
    takd: &Path,
    roots: &LiveTorRoots,
    extra_env: &[(String, String)],
) -> ChildGuard {
    let mut command = StdCommand::new(takd);
    command
        .args([
            "serve",
            "--config-root",
            &roots.server_config_root.display().to_string(),
            "--state-root",
            &roots.server_state_root.display().to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
        .env("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1000");
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let child = command.spawn().expect("spawn takd serve");
    ChildGuard { child }
}

pub fn wait_for_token(takd: &Path, roots: &LiveTorRoots) -> String {
    let output = StdCommand::new(takd)
        .args([
            "token",
            "show",
            "--state-root",
            &roots.server_state_root.display().to_string(),
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
