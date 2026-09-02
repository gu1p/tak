use crate::ContainerExecutionPlan;
use crate::container_engine::{
    ContainerEngine, ShellContainerEngineProbe, resolve_container_engine_host_platform,
    select_container_engine_with_probe,
};
use crate::worker_runtime::{ContainerLifecycleStage, RuntimeExecutionMetadata};
use anyhow::{Result, anyhow};
use std::path::Path;
use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec, ResolvedTask};
use uuid::Uuid;

mod containerized;
mod test_injection;

use containerized::{build_containerized_env_overrides, should_use_simulated_container_runtime};
use test_injection::maybe_fail_injected_container_lifecycle_stage;

pub(crate) fn resolve_runtime_execution_metadata_for_node_runtime_with_workspace(
    task: &ResolvedTask,
    node_id: &str,
    runtime: &RemoteRuntimeSpec,
    workspace_root: Option<&Path>,
) -> Result<Option<RuntimeExecutionMetadata>> {
    let (source, mounts, container_env, private_root, resource_limits) = match runtime {
        RemoteRuntimeSpec::Containerized {
            source,
            resource_limits,
        } => (source, &[][..], None, None, resource_limits),
        RemoteRuntimeSpec::ContainerizedV2 {
            source,
            mounts,
            env,
            private_root,
            resource_limits,
        } => (
            source,
            mounts.as_slice(),
            Some(env),
            Some(private_root.as_path()),
            resource_limits,
        ),
    };
    maybe_fail_injected_container_lifecycle_stage(task, node_id, ContainerLifecycleStage::Pull)?;
    let simulate_container_runtime = should_use_simulated_container_runtime();
    let engine = if simulate_container_runtime {
        ContainerEngine::Docker
    } else {
        let mut probe = ShellContainerEngineProbe;
        select_container_engine_with_probe(resolve_container_engine_host_platform(), &mut probe)
            .map_err(|err| {
                anyhow!(
                    "infra error: remote node {} container lifecycle {} failed for task {}: {}",
                    node_id,
                    ContainerLifecycleStage::Start.as_str(),
                    task.label,
                    err
                )
            })?
    };
    maybe_fail_injected_container_lifecycle_stage(task, node_id, ContainerLifecycleStage::Start)?;
    let engine_name = match engine {
        ContainerEngine::Docker => "docker".to_string(),
        ContainerEngine::Podman => "podman".to_string(),
    };
    let (runtime_source, image) = match source {
        ContainerRuntimeSourceSpec::Image { image } => ("image", image.clone()),
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile,
            build_context,
        } => {
            let image = if simulate_container_runtime {
                format!("tak-runtime-{}", Uuid::new_v4())
            } else if let Some(workspace_root) = workspace_root {
                crate::container_runtime::deterministic_dockerfile_image_tag(
                    engine,
                    workspace_root,
                    dockerfile,
                    build_context,
                )?
            } else {
                format!("tak-runtime-{}", Uuid::new_v4())
            };
            ("dockerfile", image)
        }
    };
    let mut env_overrides = build_containerized_env_overrides(
        &engine_name,
        runtime_source,
        &image,
        resource_limits.as_ref(),
    );
    if let Some(container_env) = container_env {
        env_overrides.extend(container_env.clone());
    }
    maybe_fail_injected_container_lifecycle_stage(task, node_id, ContainerLifecycleStage::Runtime)?;
    let container_plan = if simulate_container_runtime {
        None
    } else {
        Some(ContainerExecutionPlan {
            engine,
            source: source.clone(),
            image: image.clone(),
            container_user: None,
            image_cache: None,
            mounts: mounts.to_vec(),
            private_root: private_root.map(Path::to_path_buf),
            resource_limits: resource_limits.clone(),
        })
    };
    Ok(Some(RuntimeExecutionMetadata {
        kind: "containerized".to_string(),
        engine: Some(engine_name),
        node_id: node_id.to_string(),
        env_overrides,
        container_plan,
        container_identity: None,
    }))
}
