#![allow(dead_code)]
use super::container_runtime::simulated_container_runtime_env;
use super::short_daemon_paths::DaemonCommandPaths;
use super::tor_smoke::{ChildGuard, assert_success_with_log};
use std::path::Path;
use std::process::Stdio;

mod roots;

pub use roots::LiveDirectRoots;
pub fn init_direct_agent(takd: &Path, roots: &LiveDirectRoots, node_id: &str) {
    init_direct_agent_with_base_url(takd, roots, node_id, "http://127.0.0.1:0");
}

pub fn init_direct_agent_with_base_url(
    takd: &Path,
    roots: &LiveDirectRoots,
    node_id: &str,
    base_url: &str,
) {
    let paths = DaemonCommandPaths::new(&roots.server_config_root, &roots.server_state_root);
    let output = paths
        .rooted_command(takd, "init")
        .args([
            "--node-id",
            node_id,
            "--transport",
            "direct",
            "--base-url",
            base_url,
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
    let paths = DaemonCommandPaths::new(&roots.server_config_root, &roots.server_state_root);
    let mut command = paths.rooted_command(takd, "serve");
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command.env("XDG_CONFIG_HOME", paths.config_root());
    command.env("XDG_RUNTIME_DIR", paths.runtime_root());
    command.env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root());
    for (key, value) in simulated_container_runtime_env(paths.command_root()) {
        command.env(key, value);
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let child = command
        .spawn()
        .expect("spawn takd serve for direct transport");
    ChildGuard { child }
}
