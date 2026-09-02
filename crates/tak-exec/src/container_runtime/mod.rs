use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use bollard::Docker;
use bollard::container::{
    Config as ContainerConfig, CreateContainerOptions, InspectContainerOptions, LogOutput,
    LogsOptions, RemoveContainerOptions, StartContainerOptions, WaitContainerOptions,
};
use bollard::errors::Error as BollardError;
use bollard::models::HostConfig;
use futures::StreamExt;
use tak_core::model::{ContainerResourceLimitsSpec, ResolvedTask, StepDef, TaskLabel};
use uuid::Uuid;

use crate::RunCancellation;
use crate::container_engine::ContainerEngine;
use crate::step_runner::{StepRunContext, StepRunResult, resolve_cwd};
use crate::{
    ContainerExecutionPlan, OutputStream, TaskOutputObserver, TaskStatusEvent, TaskStatusPhase,
};

mod build_context;
mod execution;
mod execution_wait;
mod foundation;
mod log_stream;
mod step_spec;
mod tar_archive;
mod types;

pub(crate) use build_context::deterministic_dockerfile_image_tag;
use build_context::ensure_container_runtime_source;
use execution::run_step_in_container;
use execution_wait::{cleanup_container, wait_for_container_step};
pub(crate) use foundation::connect_container_engine;
use log_stream::{ContainerLogTask, finish_container_log_task, spawn_container_log_task};
use step_spec::build_container_step_spec;
use tar_archive::{append_tar_entry, tar_builder};
use types::{ContainerStepExecutor, ContainerStepRunContext, ContainerStepSpec};

use foundation::ensure_container_image;

pub(crate) async fn run_task_steps_in_container(
    task: &ResolvedTask,
    plan: &ContainerExecutionPlan,
    context: StepRunContext<'_>,
) -> Result<StepRunResult> {
    let client = connect_container_engine(plan.engine).await?;
    let run_context = ContainerStepRunContext {
        workspace_root: context.workspace_root,
        mounts: &plan.mounts,
        private_root: plan.private_root.as_deref(),
        task_label: context.task_label,
        task_run_id: context.task_run_id,
        attempt: context.attempt,
        output_observer: context.output_observer,
        container_user: plan.container_user.as_deref(),
        cancellation: context.cancellation,
        container_identity: context.container_identity,
        container_node_id: context.container_node_id,
        timeout_s: task.timeout_s,
    };
    let executor = ContainerStepExecutor {
        docker: &client.docker,
        engine: plan.engine,
        podman_wait_socket: client.podman_wait_socket.as_deref(),
        image: &plan.image,
        resource_limits: plan.resource_limits.as_ref(),
    };
    tokio::select! {
        result = ensure_container_runtime_source(executor.docker, context.workspace_root, plan, &run_context) => result?,
        _ = context.cancellation.cancelled() => return Err(crate::cancellation::cancelled_error()),
    }

    for step in &task.steps {
        if context.cancellation.is_cancelled() {
            return Err(crate::cancellation::cancelled_error());
        }
        let mut step_spec = build_container_step_spec(
            step,
            context.workspace_root,
            context.base_environment,
            context.runtime_env,
            plan.private_root.as_deref(),
        )?;
        apply_container_user_defaults(
            &mut step_spec,
            context.workspace_root,
            plan.container_user.as_deref(),
        );
        let status =
            run_step_in_container(&executor, &step_spec, task.timeout_s, &run_context).await?;
        if !status.success {
            return Ok(status);
        }
    }

    Ok(StepRunResult {
        success: true,
        exit_code: Some(0),
        container_oom_killed: None,
    })
}

fn apply_container_user_defaults(
    step_spec: &mut ContainerStepSpec,
    workspace_root: &Path,
    container_user: Option<&str>,
) {
    let Some(container_user) = container_user else {
        return;
    };
    if container_user_uses_numeric_uid(container_user) {
        step_spec
            .env
            .entry("HOME".to_string())
            .or_insert_with(|| workspace_root.display().to_string());
    }
}

fn container_user_uses_numeric_uid(container_user: &str) -> bool {
    let uid = container_user.split(':').next().unwrap_or_default();
    !uid.is_empty() && uid.chars().all(|value| value.is_ascii_digit())
}

#[cfg(test)]
mod execution_wait_tests;

#[cfg(test)]
mod execution_tests;

#[cfg(test)]
mod container_user_tests;
