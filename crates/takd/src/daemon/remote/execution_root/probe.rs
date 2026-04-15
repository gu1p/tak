use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use anyhow::{Context, Result, anyhow, bail};
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, StartContainerOptions,
};
use bollard::models::HostConfig;
use uuid::Uuid;

use crate::daemon::transport::{ContainerEngine, ContainerEngineProbe, select_container_engine};

use super::cache::RemoteExecRootCacheKey;
use super::client::{
    ContainerEngineClient, cleanup_container, connect_container_engine, ensure_probe_image,
    wait_for_container_exit_code,
};
use super::{PROBE_MOUNT, PROBE_SENTINEL, candidate_remote_execution_root_bases};

struct ShellContainerEngineProbe;

impl ContainerEngineProbe for ShellContainerEngineProbe {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
        let binary = match engine {
            ContainerEngine::Docker => "docker",
            ContainerEngine::Podman => "podman",
        };
        let status = StdCommand::new(binary)
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

pub(super) fn probe_remote_execution_root_candidates(
    key: &RemoteExecRootCacheKey,
) -> Result<PathBuf> {
    let key = key.clone();
    std::thread::Builder::new()
        .name("takd-exec-root-probe".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("failed to create tokio runtime for exec-root probe")?;
            runtime
                .block_on(async move { probe_remote_execution_root_candidates_async(&key).await })
        })
        .context("failed to spawn exec-root probe thread")?
        .join()
        .map_err(|_| anyhow!("remote execution root probe thread panicked"))?
}

async fn probe_remote_execution_root_candidates_async(
    key: &RemoteExecRootCacheKey,
) -> Result<PathBuf> {
    let mut engine_probe = ShellContainerEngineProbe;
    let engine = select_container_engine(&mut engine_probe)
        .context("container engine selection failed during exec-root probe")?;
    let client = connect_container_engine(engine).await?;
    let probe_image = ensure_probe_image(&client.docker).await?;

    let mut failures = Vec::new();
    for candidate in candidate_remote_execution_root_bases(key) {
        match probe_candidate(&client, engine, probe_image.image(), &candidate).await {
            Ok(()) => return Ok(candidate),
            Err(err) => failures.push(format!("{}: {err}", candidate.display())),
        }
    }

    bail!(
        "no exec-root probe candidate succeeded: {}",
        failures.join("; ")
    );
}

async fn probe_candidate(
    client: &ContainerEngineClient,
    engine: ContainerEngine,
    probe_image: &str,
    candidate: &Path,
) -> Result<()> {
    fs::create_dir_all(candidate)
        .with_context(|| format!("failed to create probe candidate {}", candidate.display()))?;
    let probe_root = candidate.join(format!("probe-{}", Uuid::new_v4()));
    fs::create_dir_all(&probe_root)
        .with_context(|| format!("failed to create probe root {}", probe_root.display()))?;
    let sentinel = probe_root.join(PROBE_SENTINEL);
    fs::write(&sentinel, b"probe")
        .with_context(|| format!("failed to write probe sentinel {}", sentinel.display()))?;

    let container_name = format!("tak-exec-root-probe-{}", Uuid::new_v4());
    let bind_mount = format!("{}:{PROBE_MOUNT}:ro", probe_root.display());
    let config = ContainerConfig {
        image: Some(probe_image.to_string()),
        cmd: Some(vec![
            "test".to_string(),
            "-f".to_string(),
            format!("{PROBE_MOUNT}/{PROBE_SENTINEL}"),
        ]),
        attach_stdout: Some(false),
        attach_stderr: Some(false),
        tty: Some(false),
        host_config: Some(HostConfig {
            binds: Some(vec![bind_mount]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let create_result = client
        .docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            config,
        )
        .await;
    let container_id = match create_result {
        Ok(created) => created.id,
        Err(err) => {
            let _ = fs::remove_dir_all(&probe_root);
            return Err(err).context("failed to create probe container");
        }
    };

    let result = async {
        client
            .docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .context("failed to start probe container")?;
        let exit_code = wait_for_container_exit_code(client, engine, &container_id).await?;
        if exit_code != 0 {
            bail!("probe container exited with status {exit_code}");
        }
        Ok(())
    }
    .await;

    let _ = cleanup_container(&client.docker, &container_id).await;
    let _ = fs::remove_dir_all(&probe_root);
    result
}
