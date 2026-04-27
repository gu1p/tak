use super::remote_models::{ContainerLifecycleStage, RuntimeExecutionMetadata, TaskPlacement};
use super::{ContainerExecutionPlan, PlacementMode, StrictRemoteTarget};
use crate::container_engine::{
    ContainerEngine, ShellContainerEngineProbe, resolve_container_engine_host_platform,
    select_container_engine_with_probe,
};
use anyhow::{Result, anyhow, bail};
use std::collections::BTreeMap;
use std::env;
use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec, ResolvedTask};
use uuid::Uuid;

pub(crate) fn resolve_runtime_execution_metadata(
    task: &ResolvedTask,
    placement: &TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    if placement.placement_mode == PlacementMode::Local {
        return resolve_runtime_execution_metadata_for_node_runtime(
            task,
            "local",
            placement
                .local
                .as_ref()
                .and_then(|local| local.runtime.as_ref()),
        );
    }
    let Some(target) = placement.strict_remote_target.as_ref() else {
        return Ok(None);
    };
    resolve_runtime_execution_metadata_for_target(task, target)
}

pub(crate) fn resolve_runtime_execution_metadata_for_target(
    task: &ResolvedTask,
    target: &StrictRemoteTarget,
) -> Result<Option<RuntimeExecutionMetadata>> {
    resolve_runtime_execution_metadata_for_node_runtime(
        task,
        &target.node_id,
        target.runtime.as_ref(),
    )
}

pub(crate) fn resolve_runtime_execution_metadata_for_node_runtime(
    task: &ResolvedTask,
    node_id: &str,
    runtime: Option<&RemoteRuntimeSpec>,
) -> Result<Option<RuntimeExecutionMetadata>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };
    match runtime {
        RemoteRuntimeSpec::Containerized { source } => {
            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Pull,
            )?;
            let simulate_container_runtime = should_use_simulated_container_runtime();
            let engine = if simulate_container_runtime {
                ContainerEngine::Docker
            } else {
                let mut probe = ShellContainerEngineProbe;
                select_container_engine_with_probe(
                    resolve_container_engine_host_platform(),
                    &mut probe,
                )
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
            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Start,
            )?;
            let engine_name = match engine {
                ContainerEngine::Docker => "docker".to_string(),
                ContainerEngine::Podman => "podman".to_string(),
            };
            let (runtime_source, image) = match source {
                ContainerRuntimeSourceSpec::Image { image } => ("image", image.clone()),
                ContainerRuntimeSourceSpec::Dockerfile { .. } => {
                    ("dockerfile", format!("tak-runtime-{}", Uuid::new_v4()))
                }
            };
            let mut env_overrides = BTreeMap::new();
            env_overrides.insert("TAK_RUNTIME".to_string(), "containerized".to_string());
            env_overrides.insert("TAK_RUNTIME_ENGINE".to_string(), engine_name.clone());
            env_overrides.insert("TAK_RUNTIME_SOURCE".to_string(), runtime_source.to_string());
            env_overrides.insert("TAK_CONTAINER_IMAGE".to_string(), image.clone());
            env_overrides.insert(
                "TAK_REMOTE_RUNTIME".to_string(),
                "containerized".to_string(),
            );
            env_overrides.insert("TAK_REMOTE_ENGINE".to_string(), engine_name.clone());
            env_overrides.insert("TAK_REMOTE_CONTAINER_IMAGE".to_string(), image.clone());
            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Runtime,
            )?;
            let container_plan = if simulate_container_runtime {
                None
            } else {
                Some(ContainerExecutionPlan {
                    engine,
                    source: source.clone(),
                    image: image.clone(),
                    container_user: None,
                })
            };
            Ok(Some(RuntimeExecutionMetadata {
                kind: "containerized".to_string(),
                engine: Some(engine_name),
                env_overrides,
                container_plan,
            }))
        }
    }
}

fn should_use_simulated_container_runtime() -> bool {
    env::var("TAK_TEST_HOST_PLATFORM").is_ok()
        || env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_ok()
}

pub(crate) fn maybe_fail_injected_container_lifecycle_stage(
    task: &ResolvedTask,
    node_id: &str,
    stage: ContainerLifecycleStage,
) -> Result<()> {
    let Some(injected_stage) = test_injected_container_lifecycle_stage(node_id) else {
        return Ok(());
    };
    if injected_stage != stage {
        return Ok(());
    }

    bail!(
        "infra error: remote node {} container lifecycle {} failed for task {}: simulated deterministic failure",
        node_id,
        stage.as_str(),
        task.label
    );
}

pub(crate) fn test_injected_container_lifecycle_stage(
    node_id: &str,
) -> Option<ContainerLifecycleStage> {
    let configured = env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").ok()?;
    for entry in configured.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let Some((entry_node, raw_stage)) = entry.split_once(':') else {
            continue;
        };
        if entry_node.trim() != node_id {
            continue;
        }

        let stage = raw_stage
            .trim()
            .split(':')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        return match stage.as_str() {
            "pull" => Some(ContainerLifecycleStage::Pull),
            "start" => Some(ContainerLifecycleStage::Start),
            "runtime" => Some(ContainerLifecycleStage::Runtime),
            _ => None,
        };
    }

    None
}
