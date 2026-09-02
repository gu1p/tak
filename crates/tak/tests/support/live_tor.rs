#![allow(dead_code)]

use std::path::Path;
use std::process::Stdio;

pub use super::live_tor_roots::LiveTorRoots;
use super::short_daemon_paths::DaemonCommandPaths;
use super::tor_smoke::{ChildGuard, assert_success, assert_success_with_log, tak_command};

pub fn init_tor_agent(takd: &Path, roots: &LiveTorRoots, node_id: &str) {
    let paths = DaemonCommandPaths::new(&roots.server_config_root, &roots.server_state_root);
    let output = paths
        .rooted_command(takd, "init")
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

pub fn spawn_tor_agent(takd: &Path, roots: &LiveTorRoots) -> ChildGuard {
    spawn_tor_agent_with_env(takd, roots, &[])
}

pub fn spawn_tor_agent_with_env(
    takd: &Path,
    roots: &LiveTorRoots,
    extra_env: &[(String, String)],
) -> ChildGuard {
    let paths = DaemonCommandPaths::new(&roots.server_config_root, &roots.server_state_root);
    let mut command = paths.rooted_command(takd, "serve");
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command.env("XDG_CONFIG_HOME", paths.config_root());
    command.env("XDG_RUNTIME_DIR", paths.runtime_root());
    command
        .env("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1000")
        .env("TAKD_TOR_RECOVERY_PROBE_TIMEOUT_MS", "300000")
        .env("TAKD_TOR_RECOVERY_PROBE_BACKOFF_MS", "1000");
    command.env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let child = command.spawn().expect("spawn takd serve");
    ChildGuard { child }
}

pub fn wait_for_token(takd: &Path, roots: &LiveTorRoots) -> String {
    let paths = DaemonCommandPaths::new(&roots.server_config_root, &roots.server_state_root);
    let output = paths
        .state_command(takd, &["token", "show"])
        .args(["--wait", "--timeout-secs", "360"])
        .output()
        .expect("run takd token show --wait");
    assert_success_with_log(&output, "takd token show --wait", &roots.service_log_path());
    String::from_utf8(output.stdout)
        .expect("token stdout utf8")
        .trim()
        .to_string()
}
