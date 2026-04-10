#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

use super::tor_smoke::{ChildGuard, assert_success_with_log};

pub struct LiveDirectRoots {
    pub server_config_root: PathBuf,
    pub server_state_root: PathBuf,
    pub client_config_root: PathBuf,
}

impl LiveDirectRoots {
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

pub fn init_direct_agent(takd: &Path, roots: &LiveDirectRoots, node_id: &str) {
    let output = StdCommand::new(takd)
        .args([
            "init",
            "--config-root",
            &roots.server_config_root.display().to_string(),
            "--state-root",
            &roots.server_state_root.display().to_string(),
            "--node-id",
            node_id,
            "--transport",
            "direct",
            "--base-url",
            "http://127.0.0.1:0",
            "--pool",
            "build",
            "--pool",
            "test",
            "--tag",
            "builder",
            "--capability",
            "linux",
        ])
        .output()
        .expect("run takd init for direct transport");
    assert_success_with_log(
        &output,
        "takd init --transport direct",
        &roots.service_log_path(),
    );
}

pub fn spawn_direct_agent(takd: &Path, roots: &LiveDirectRoots) -> ChildGuard {
    spawn_direct_agent_with_env(takd, roots, &[])
}

pub fn spawn_direct_agent_with_env(
    takd: &Path,
    roots: &LiveDirectRoots,
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
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let child = command
        .spawn()
        .expect("spawn takd serve for direct transport");
    ChildGuard { child }
}
