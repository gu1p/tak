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
use bollard::errors::Error as BollardError;
use bollard::models::HostConfig;
use futures::StreamExt;
use tak_core::model::{ResolvedTask, StepDef, TaskLabel};
use uuid::Uuid;

use crate::container_engine::ContainerEngine;
use crate::step_runner::{StepRunResult, resolve_cwd};
use crate::{ContainerExecutionPlan, OutputStream, TaskOutputObserver};

mod build_context;
mod execution;
mod foundation;
mod log_stream;
mod tar_archive;

use build_context::ensure_container_runtime_source;
use execution::run_step_in_container;
use foundation::connect_container_engine;
use log_stream::{finish_container_log_task, spawn_container_log_task};
use tar_archive::{append_tar_entry, tar_builder};

use foundation::ensure_container_image;

#[derive(Debug)]
pub(crate) struct ContainerStepSpec {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
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
    plan: &ContainerExecutionPlan,
    runtime_env: Option<&BTreeMap<String, String>>,
    attempt: u32,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
) -> Result<StepRunResult> {
    let client = connect_container_engine(plan.engine).await?;
    ensure_container_runtime_source(&client.docker, workspace_root, plan).await?;
    let run_context = ContainerStepRunContext {
        workspace_root,
        task_label: &task.label,
        attempt,
        output_observer,
    };

    for step in &task.steps {
        let step_spec = build_container_step_spec(step, workspace_root, runtime_env)?;
        let status = run_step_in_container(
            &client.docker,
            plan.engine,
            client.podman_wait_socket.as_deref(),
            &plan.image,
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

#[cfg(test)]
mod execution_wait_tests;
