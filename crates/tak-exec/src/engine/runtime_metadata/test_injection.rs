use super::*;

use anyhow::bail;

pub(super) fn maybe_fail_injected_container_lifecycle_stage(
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
