fn resolve_runtime_execution_metadata(
    task: &ResolvedTask,
    placement: &TaskPlacement,
) -> Result<Option<RuntimeExecutionMetadata>> {
    if placement.placement_mode != PlacementMode::Remote {
        return Ok(None);
    }

    let Some(target) = placement.strict_remote_target.as_ref() else {
        return Ok(None);
    };
    resolve_runtime_execution_metadata_for_target(task, target)
}

fn resolve_runtime_execution_metadata_for_target(
    task: &ResolvedTask,
    target: &StrictRemoteTarget,
) -> Result<Option<RuntimeExecutionMetadata>> {
    resolve_runtime_execution_metadata_for_node_runtime(
        task,
        &target.node_id,
        target.runtime.as_ref(),
    )
}

fn resolve_runtime_execution_metadata_for_node_runtime(
    task: &ResolvedTask,
    node_id: &str,
    runtime: Option<&RemoteRuntimeSpec>,
) -> Result<Option<RuntimeExecutionMetadata>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };

    match runtime {
        RemoteRuntimeSpec::Containerized { image } => {
            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Pull,
            )?;

            let mut probe = ShellContainerEngineProbe;
            let engine = select_container_engine_with_probe(
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
            })?;

            maybe_fail_injected_container_lifecycle_stage(
                task,
                node_id,
                ContainerLifecycleStage::Start,
            )?;

            let engine_name = match engine {
                ContainerEngine::Docker => "docker".to_string(),
                ContainerEngine::Podman => "podman".to_string(),
            };

            let mut env_overrides = BTreeMap::new();
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

            let container_plan = if should_use_simulated_container_runtime() {
                None
            } else {
                Some(ContainerExecutionPlan {
                    engine,
                    image: image.clone(),
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

fn maybe_fail_injected_container_lifecycle_stage(
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

fn test_injected_container_lifecycle_stage(node_id: &str) -> Option<ContainerLifecycleStage> {
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
