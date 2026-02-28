use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use futures::StreamExt;
use tak_core::model::{ResolvedTask, StepDef};
use uuid::Uuid;

use crate::container_engine::{ContainerEngine, engine_name, podman_socket_candidates_from_env};
use crate::step_runner::{StepRunResult, resolve_cwd};

#[derive(Debug)]
pub(crate) struct ContainerStepSpec {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
}

pub(crate) async fn run_task_steps_in_container(
    task: &ResolvedTask,
    workspace_root: &Path,
    engine: ContainerEngine,
    image: &str,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<StepRunResult> {
    let docker = connect_container_engine(engine).await?;
    ensure_container_image(&docker, image).await?;

    for step in &task.steps {
        let step_spec = build_container_step_spec(step, workspace_root, runtime_env)?;
        let status =
            run_step_in_container(&docker, image, &step_spec, task.timeout_s, workspace_root)
                .await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
    })
}

async fn connect_container_engine(engine: ContainerEngine) -> Result<Docker> {
    let docker = match engine {
        ContainerEngine::Docker => Docker::connect_with_local_defaults().context(
            "infra error: container lifecycle start failed: docker client connect failed",
        )?,
        ContainerEngine::Podman => connect_podman_client()?,
    };
    docker.ping().await.with_context(|| {
        format!(
            "infra error: container lifecycle start failed: {} ping failed",
            engine_name(engine)
        )
    })?;
    Ok(docker)
}

fn connect_podman_client() -> Result<Docker> {
    for socket in podman_socket_candidates_from_env() {
        let socket_path = socket.strip_prefix("unix://").unwrap_or(socket.as_str());
        if let Ok(client) =
            Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)
        {
            return Ok(client);
        }
    }
    bail!("infra error: container lifecycle start failed: no podman socket available");
}

async fn ensure_container_image(docker: &Docker, image: &str) -> Result<()> {
    match docker.inspect_image(image).await {
        Ok(_) => return Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(err) => {
            return Err(err)
                .context("infra error: container lifecycle pull failed: inspect image failed");
        }
    }

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(item) = stream.next().await {
        item.context("infra error: container lifecycle pull failed")?;
    }
    Ok(())
}

pub(crate) fn build_container_step_spec(
    step: &StepDef,
    workspace_root: &Path,
    runtime_env: Option<&BTreeMap<String, String>>,
) -> Result<ContainerStepSpec> {
    match step {
        StepDef::Cmd { argv, cwd, env } => {
            if argv.is_empty() {
                bail!("cmd step requires a non-empty argv");
            }
            let mut env_map = BTreeMap::new();
            if let Some(runtime_env) = runtime_env {
                env_map.extend(runtime_env.clone());
            }
            env_map.extend(env.clone());
            Ok(ContainerStepSpec {
                argv: argv.clone(),
                cwd: resolve_cwd(workspace_root, cwd),
                env: env_map,
            })
        }
        StepDef::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => {
            let mut full_argv = Vec::with_capacity(argv.len() + 2);
            if let Some(interpreter) = interpreter {
                full_argv.push(interpreter.clone());
                full_argv.push(path.clone());
            } else {
                full_argv.push(path.clone());
            }
            full_argv.extend(argv.clone());

            let mut env_map = BTreeMap::new();
            if let Some(runtime_env) = runtime_env {
                env_map.extend(runtime_env.clone());
            }
            env_map.extend(env.clone());
            Ok(ContainerStepSpec {
                argv: full_argv,
                cwd: resolve_cwd(workspace_root, cwd),
                env: env_map,
            })
        }
    }
}

async fn run_step_in_container(
    docker: &Docker,
    image: &str,
    step: &ContainerStepSpec,
    timeout_s: Option<u64>,
    workspace_root: &Path,
) -> Result<StepRunResult> {
    let container_name = format!("tak-step-{}", Uuid::new_v4());
    let bind_mount = format!(
        "{}:{}:rw",
        workspace_root.display(),
        workspace_root.display()
    );
    let env = step
        .env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();

    let config = ContainerConfig {
        image: Some(image.to_string()),
        cmd: Some(step.argv.clone()),
        env: Some(env),
        working_dir: Some(step.cwd.to_string_lossy().to_string()),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        host_config: Some(HostConfig {
            binds: Some(vec![bind_mount]),
            ..Default::default()
        }),
        ..Default::default()
    };

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            config,
        )
        .await
        .context("infra error: container lifecycle start failed: create container failed")?;
    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await
        .context("infra error: container lifecycle start failed: start container failed")?;

    let wait_result = wait_for_container_exit_code(docker, &container_name);
    let status = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), wait_result).await {
            Ok(result) => result?,
            Err(_) => {
                let _ = cleanup_container(docker, &container_name).await;
                return Ok(StepRunResult {
                    success: false,
                    exit_code: None,
                });
            }
        }
    } else {
        wait_result.await?
    };

    let _ = cleanup_container(docker, &container_name).await;
    Ok(StepRunResult {
        success: status == 0,
        exit_code: Some(status),
    })
}

async fn wait_for_container_exit_code(docker: &Docker, container_name: &str) -> Result<i32> {
    let mut stream = docker.wait_container(container_name, None::<WaitContainerOptions<String>>);
    let Some(result) = stream.next().await else {
        bail!("infra error: container lifecycle runtime failed: wait stream ended unexpectedly");
    };
    let result = result
        .context("infra error: container lifecycle runtime failed: waiting for container failed")?;
    let code = i32::try_from(result.status_code).unwrap_or(1);
    Ok(code)
}

async fn cleanup_container(docker: &Docker, container_name: &str) -> Result<()> {
    let _ = docker
        .remove_container(
            container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    Ok(())
}
