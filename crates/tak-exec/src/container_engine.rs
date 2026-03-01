use std::env;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContainerEngine {
    Docker,
    Podman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostPlatform {
    MacOs,
    Other,
}

impl HostPlatform {
    pub(crate) fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

pub(crate) trait ContainerEngineProbe {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String>;
}

pub(crate) fn select_container_engine_with_probe(
    platform: HostPlatform,
    probe: &mut impl ContainerEngineProbe,
) -> Result<ContainerEngine> {
    if probe.probe(ContainerEngine::Docker).is_ok() {
        return Ok(ContainerEngine::Docker);
    }

    if platform == HostPlatform::MacOs && probe.probe(ContainerEngine::Podman).is_ok() {
        return Ok(ContainerEngine::Podman);
    }

    let attempted = if platform == HostPlatform::MacOs {
        "docker, podman"
    } else {
        "docker"
    };
    bail!("no container engine available; attempted probes: {attempted}");
}

pub(crate) struct ShellContainerEngineProbe;

impl ContainerEngineProbe for ShellContainerEngineProbe {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
        let binary = match engine {
            ContainerEngine::Docker => "docker",
            ContainerEngine::Podman => "podman",
        };

        let status = std::process::Command::new(binary)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!(
                "engine probe `{binary}` exited with status {status}"
            )),
            Err(err) => Err(err.to_string()),
        }
    }
}

pub(crate) fn resolve_container_engine_host_platform() -> HostPlatform {
    match env::var("TAK_TEST_HOST_PLATFORM")
        .ok()
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("macos") => HostPlatform::MacOs,
        Some("other") => HostPlatform::Other,
        _ => HostPlatform::current(),
    }
}

pub(crate) fn engine_name(engine: ContainerEngine) -> &'static str {
    match engine {
        ContainerEngine::Docker => "docker",
        ContainerEngine::Podman => "podman",
    }
}

pub(crate) fn podman_socket_candidates_from_inputs(
    explicit: Option<&str>,
    runtime_dir: Option<&str>,
    uid: Option<&str>,
) -> Vec<String> {
    let mut sockets = Vec::new();

    if let Some(explicit) = explicit {
        let explicit = normalize_podman_socket(explicit);
        if let Some(explicit) = explicit {
            sockets.push(explicit);
        }
    }

    if let Some(runtime_dir) = runtime_dir {
        let runtime_dir = runtime_dir.trim();
        if !runtime_dir.is_empty() {
            sockets.push(format!("unix://{runtime_dir}/podman/podman.sock"));
        }
    }

    if let Some(uid) = uid {
        let uid = uid.trim();
        if !uid.is_empty() {
            sockets.push(format!("unix:///run/user/{uid}/podman/podman.sock"));
        }
    }

    sockets.push("unix:///run/podman/podman.sock".to_string());
    sockets
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

pub(crate) fn podman_socket_candidates_from_env() -> Vec<String> {
    let explicit = env::var("TAK_PODMAN_SOCKET").ok();
    let runtime_dir = env::var("XDG_RUNTIME_DIR").ok();
    let uid = env::var("UID").ok();
    let mut sockets = podman_socket_candidates_from_inputs(
        explicit.as_deref(),
        runtime_dir.as_deref(),
        uid.as_deref(),
    );
    if let Ok(tmpdir) = env::var("TMPDIR") {
        let tmpdir = tmpdir.trim().trim_end_matches('/');
        if !tmpdir.is_empty() {
            sockets.push(format!(
                "unix://{tmpdir}/podman/podman-machine-default-api.sock"
            ));
        }
    }
    sockets
}
