use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, LogOutput, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use futures::StreamExt;
use tak_core::model::{ResolvedTask, StepDef, TaskLabel};
use uuid::Uuid;

use crate::container_engine::{ContainerEngine, engine_name, podman_socket_candidates_from_env};
use crate::step_runner::{StepRunResult, resolve_cwd};
use crate::{OutputStream, TaskOutputObserver};

#[derive(Debug)]
pub(crate) struct ContainerStepSpec {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Debug)]
struct ContainerEngineClient {
    docker: Docker,
    podman_wait_socket: Option<String>,
}

struct ContainerStepRunContext<'a> {
    workspace_root: &'a Path,
    task_label: &'a TaskLabel,
    attempt: u32,
    output_observer: Option<&'a Arc<dyn TaskOutputObserver>>,
}

pub(crate) async fn run_task_steps_in_container(
    task: &ResolvedTask,
    workspace_root: &Path,
    engine: ContainerEngine,
    image: &str,
    runtime_env: Option<&BTreeMap<String, String>>,
    attempt: u32,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
) -> Result<StepRunResult> {
    let client = connect_container_engine(engine).await?;
    ensure_container_image(&client.docker, image).await?;
    let run_context = ContainerStepRunContext {
        workspace_root,
        task_label: &task.label,
        attempt,
        output_observer,
    };

    for step in &task.steps {
        let step_spec = build_container_step_spec(step, workspace_root, runtime_env)?;
        let status =
            run_step_in_container(
                &client.docker,
                engine,
                client.podman_wait_socket.as_deref(),
                image,
                &step_spec,
                task.timeout_s,
                &run_context,
            )
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

async fn connect_container_engine(engine: ContainerEngine) -> Result<ContainerEngineClient> {
    match engine {
        ContainerEngine::Docker => {
            let docker = Docker::connect_with_local_defaults().context(
            "infra error: container lifecycle start failed: docker client connect failed",
        )?;
            docker.ping().await.with_context(|| {
            format!(
                "infra error: container lifecycle start failed: {} ping failed",
                engine_name(engine)
            )
        })?;
            Ok(ContainerEngineClient {
                docker,
                podman_wait_socket: None,
            })
        }
        ContainerEngine::Podman => connect_podman_client().await,
    }
}

async fn connect_podman_client() -> Result<ContainerEngineClient> {
    for socket in podman_socket_candidates_from_env() {
        let socket_path = socket.strip_prefix("unix://").unwrap_or(socket.as_str());
        let Ok(client) = Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)
        else {
            continue;
        };
        if client.ping().await.is_ok() {
            return Ok(ContainerEngineClient {
                docker: client,
                podman_wait_socket: Some(socket),
            });
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
