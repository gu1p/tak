use std::process::Command as StdCommand;

use anyhow::{Context, Result, bail};

use crate::daemon::remote::RemoteRuntimeConfig;

pub(super) fn podman_socket_candidates(config: &RemoteRuntimeConfig) -> Vec<String> {
    let explicit = config.podman_socket();
    let runtime_dir = config.runtime_dir();
    let uid = config.uid();
    let mut sockets = Vec::new();

    if let Some(explicit) = explicit.and_then(normalize_podman_socket) {
        sockets.push(explicit);
    }
    if let Some(runtime_dir) = runtime_dir.map(str::to_string) {
        let runtime_dir = runtime_dir.trim();
        if !runtime_dir.is_empty() {
            sockets.push(format!("unix://{runtime_dir}/podman/podman.sock"));
        }
    }
    if let Some(uid) = uid.map(str::to_string) {
        let uid = uid.trim();
        if !uid.is_empty() {
            sockets.push(format!("unix:///run/user/{uid}/podman/podman.sock"));
        }
    }

    sockets.push("unix:///run/podman/podman.sock".to_string());
    let tmpdir = config.temp_dir().display().to_string();
    let tmpdir = tmpdir.trim().trim_end_matches('/');
    if !tmpdir.is_empty() {
        sockets.push(format!(
            "unix://{tmpdir}/podman/podman-machine-default-api.sock"
        ));
    }
    sockets
}

pub(super) async fn wait_for_container_exit_code_via_cli(
    podman_wait_socket: Option<&str>,
    container_name: &str,
) -> Result<i32> {
    let mut command = StdCommand::new("podman");
    if let Some(socket) = podman_wait_socket {
        command.args(["--url", socket]);
    }
    let output = command
        .args(["wait", container_name])
        .output()
        .context("failed to launch podman wait during exec-root probe")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("podman wait failed during exec-root probe: {stderr}");
    }
    let value = String::from_utf8_lossy(&output.stdout);
    value
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .parse::<i32>()
        .context("invalid podman wait exit code during exec-root probe")
}

fn normalize_podman_socket(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("unix://") {
        return Some(value.to_string());
    }
    if value.starts_with('/') {
        return Some(format!("unix://{value}"));
    }
    Some(value.to_string())
}
