use std::path::Path;

use anyhow::{Result, anyhow};
use tak_core::model::ResolvedTask;

use super::TaskOutputObserver;
use super::placement::{PlacementCandidate, resolve_task_placement_candidates};
use super::preflight_fallback::preflight_ordered_remote_target;
use super::remote_models::TaskPlacement;
use super::remote_selection::ordered_remote_targets_for_attempt;

pub(crate) async fn preflight_task_placement(
    task: &ResolvedTask,
    workspace_root: &Path,
    task_run_id: &str,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<TaskPlacement> {
    let mut failures = Vec::new();
    for candidate in resolve_task_placement_candidates(task, workspace_root)? {
        let mut placement = match candidate {
            PlacementCandidate::Ready(placement) => *placement,
            PlacementCandidate::Unavailable(err) => {
                failures.push(err);
                continue;
            }
        };
        if placement.ordered_remote_targets.is_empty() {
            return Ok(placement);
        }
        let ordered = ordered_remote_targets_for_attempt(
            &placement.ordered_remote_targets,
            placement.remote_selection,
            &task.label.to_string(),
            task_run_id,
            attempt,
        );
        let selected = match preflight_ordered_remote_target(task, &ordered, output_observer).await
        {
            Ok(selected) => selected,
            Err(err) => {
                failures.push(err);
                continue;
            }
        };
        placement.ordered_remote_targets = ordered;
        placement.remote_node_id = Some(selected.node_id.clone());
        placement.strict_remote_target = Some(selected);
        return Ok(placement);
    }

    placement_candidates_exhausted(task, failures)
}

fn placement_candidates_exhausted(
    task: &ResolvedTask,
    failures: Vec<anyhow::Error>,
) -> Result<TaskPlacement> {
    if failures.len() == 1 {
        return Err(failures.into_iter().next().expect("single failure"));
    }
    let details = failures
        .into_iter()
        .map(|err| err.to_string())
        .collect::<Vec<_>>()
        .join("; ");
    Err(anyhow!(
        "no execution_policy placements were available for task {}: {}",
        task.label,
        details
    ))
}
