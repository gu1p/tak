#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command as StdCommand, Output, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use uuid::Uuid;

use super::e2e_harness::wait_for_http_ready;

fn container_cli() -> Option<&'static str> {
    for candidate in ["docker", "podman"] {
        let status = StdCommand::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return Some(candidate);
        }
    }
    None
}

pub(super) fn docker_prereq_reason() -> Option<String> {
    let Some(cli) = container_cli() else {
        return Some("no container engine CLI available (`docker` or `podman`)".into());
    };
    let daemon_ok = StdCommand::new(cli)
        .args(["version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !daemon_ok {
        return Some("container engine daemon is not reachable for recursive scenario".into());
    }
    None
}

pub(super) struct RemoteTakdContainer {
    cli: String,
    id: String,
}

impl Drop for RemoteTakdContainer {
    fn drop(&mut self) {
        let _ = StdCommand::new(&self.cli)
            .args(["rm", "-f", &self.id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

impl RemoteTakdContainer {
    pub(super) fn logs(&self) -> String {
        let output = StdCommand::new(&self.cli).args(["logs", &self.id]).output();
        match output {
            Ok(output) => {
                let mut merged = String::new();
                if !output.stdout.is_empty() {
                    merged.push_str(&String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    if !merged.is_empty() {
                        merged.push('\n');
                    }
                    merged.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                merged
            }
            Err(error) => format!("failed to fetch container logs: {error}"),
        }
    }
}

pub(super) fn spawn_containerized_remote_takd(
    runner_image: &str,
    repo_root: &Path,
    service_root: &Path,
    remote_port: u16,
) -> Result<RemoteTakdContainer> {
    let cli = container_cli().ok_or_else(|| anyhow!("no container engine CLI available"))?;
    let id = format!("tak-recursive-{}", Uuid::new_v4().simple());
    let remote_exec_root = service_root.join("exec-root");
    fs::create_dir_all(&remote_exec_root)
        .with_context(|| format!("create {}", remote_exec_root.display()))?;
    let cache = repo_root.join("target/recursive-e2e-cache");
    let cargo_home = cache.join("cargo-home");
    let cargo_target = cache.join("cargo-target");
    fs::create_dir_all(&cargo_home).with_context(|| format!("create {}", cargo_home.display()))?;
    fs::create_dir_all(&cargo_target)
        .with_context(|| format!("create {}", cargo_target.display()))?;
    let mut launch = StdCommand::new(cli);
    launch.args(["run", "-d", "--privileged", "--name", &id]);
    if cli == "podman" {
        launch.args(["--security-opt", "label=disable"]);
    }
    let status = launch
        .args(["-p", &format!("{remote_port}:{remote_port}")])
        .args(["-v", &format!("{}:/repo:ro", repo_root.display())])
        .args(["-v", &format!("{}:/service", service_root.display())])
        .args(["-v", &format!("{}:/cargo-home", cargo_home.display())])
        .args(["-v", &format!("{}:/cargo-target", cargo_target.display())])
        .args(["-e", "TAKD_SOCKET=/service/remote-worker.sock"])
        .args(["-e", "TAKD_DB_PATH=/service/remote-worker.sqlite"])
        .args(["-e", "DOCKER_HOST=unix:///run/podman/docker-probe.sock"])
        .args(["-e", "TAK_PODMAN_SOCKET=unix:///run/podman/podman.sock"])
        .args(["-e", "TAK_TEST_HOST_PLATFORM=macos"])
        .args(["-e", "CARGO_HOME=/cargo-home"])
        .args(["-e", "CARGO_TARGET_DIR=/cargo-target"])
        .args(["-e", "CARGO_BUILD_JOBS=1"])
        .args(["-e", "RUSTFLAGS=-Cdebuginfo=0"])
        .args([
            "-e",
            &format!("TAKD_REMOTE_V1_BIND_ADDR=0.0.0.0:{remote_port}"),
        ])
        .args(["-e", "TAKD_REMOTE_EXEC_ROOT=/service/exec-root"])
        .arg(runner_image)
        .args([
            "sh",
            "-c",
            "mkdir -p /run/podman; \
podman system service --time=0 unix:///run/podman/podman.sock >/service/podman-service.log 2>&1 & \
for i in $(seq 1 120); do podman --url unix:///run/podman/podman.sock info >/dev/null 2>&1 && break; sleep 1; done; \
podman --url unix:///run/podman/podman.sock info >/dev/null 2>&1 || { cat /service/podman-service.log >&2; exit 47; }; \
exec cargo run --quiet --manifest-path /repo/Cargo.toml -p tak -- daemon start",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("start containerized remote takd")?;
    if !status.success() {
        bail!("failed to launch containerized remote takd");
    }
    if let Err(err) = wait_for_http_ready(remote_port, "/v1/node/status", Duration::from_secs(900))
    {
        let logs = StdCommand::new(cli).args(["logs", &id]).output();
        let logs_text = logs
            .ok()
            .map(|out| {
                let mut merged = String::new();
                if !out.stdout.is_empty() {
                    merged.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    if !merged.is_empty() {
                        merged.push('\n');
                    }
                    merged.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                merged
            })
            .unwrap_or_else(|| "<failed to fetch container logs>".to_string());
        bail!(
            "wait for containerized remote takd HTTP readiness: {err}\ncontainer_id={id}\ncontainer_logs:\n{logs_text}"
        );
    }
    Ok(RemoteTakdContainer {
        cli: cli.to_string(),
        id,
    })
}

pub(super) fn run_tak_via_docker_container(
    runner_image: &str,
    repo_root: &Path,
    workspace: &Path,
    args: &[&str],
    env: &BTreeMap<String, String>,
) -> Result<Output> {
    let cli = container_cli().ok_or_else(|| anyhow!("no container engine CLI available"))?;
    let cache = repo_root.join("target/recursive-e2e-cache");
    let cargo_home = cache.join("cargo-home");
    let cargo_target = cache.join("cargo-target");
    fs::create_dir_all(&cargo_home).with_context(|| format!("create {}", cargo_home.display()))?;
    fs::create_dir_all(&cargo_target)
        .with_context(|| format!("create {}", cargo_target.display()))?;

    let mut cmd = StdCommand::new(cli);
    cmd.args(["run", "--rm"]);
    if cfg!(target_os = "linux") {
        cmd.args(["--add-host", "host.docker.internal:host-gateway"]);
    }
    cmd.args(["-v", &format!("{}:/repo:ro", repo_root.display())]);
    cmd.args(["-v", &format!("{}:/workspace", workspace.display())]);
    cmd.args(["-v", &format!("{}:/cargo-home", cargo_home.display())]);
    cmd.args(["-v", &format!("{}:/cargo-target", cargo_target.display())]);
    cmd.args(["-w", "/workspace"]);

    let mut env_all = env.clone();
    env_all.insert("TAKD_SOCKET".into(), "/workspace/.missing-takd.sock".into());
    env_all.insert("CARGO_HOME".into(), "/cargo-home".into());
    env_all.insert("CARGO_TARGET_DIR".into(), "/cargo-target".into());
    env_all.insert("CARGO_BUILD_JOBS".into(), "1".into());
    env_all.insert("RUSTFLAGS".into(), "-Cdebuginfo=0".into());
    for (key, value) in env_all {
        cmd.args(["-e", &format!("{key}={value}")]);
    }

    cmd.arg(runner_image)
        .args([
            "cargo",
            "run",
            "--quiet",
            "-j",
            "1",
            "--manifest-path",
            "/repo/Cargo.toml",
            "-p",
            "tak",
            "--",
        ])
        .args(args);
    cmd.output()
        .with_context(|| format!("run containerized tak command: {}", args.join(" ")))
}
