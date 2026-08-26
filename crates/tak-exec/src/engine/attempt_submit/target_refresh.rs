use anyhow::Result;
use tak_core::model::ResolvedTask;

use crate::engine::TaskOutputObserver;
use crate::engine::preflight_fallback::preflight_ordered_remote_target;
use crate::engine::remote_models::{StrictRemoteTarget, TaskPlacement};
use crate::engine::remote_selection::SharedRemoteSelectionState;

pub(super) async fn refresh_remote_target_for_attempt(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
    task_run_id: &str,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    remote_selection_state: &SharedRemoteSelectionState,
) -> Result<()> {
    if attempt == 1 && placement.strict_remote_target.is_some() {
        return Ok(());
    }
    if placement.ordered_remote_targets.is_empty()
        && placement
            .strict_remote_target
            .as_ref()
            .is_some_and(StrictRemoteTarget::is_daemon_tor_placement)
    {
        return Ok(());
    }
    let failed_node_ids = placement
        .infrastructure_failures
        .iter()
        .map(|failure| failure.node_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let eligible_targets = placement
        .ordered_remote_targets
        .iter()
        .filter(|target| !failed_node_ids.contains(target.node_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let ordered = remote_selection_state.reserve_ordered_targets_for_attempt(
        &eligible_targets,
        placement.remote_selection,
        &task.label.to_string(),
        task_run_id,
        attempt,
    );
    let reserved_node_id = ordered.first().map(|target| target.node_id.clone());
    let selected = match preflight_ordered_remote_target(
        task,
        &ordered,
        placement.remote_selection,
        output_observer,
    )
    .await
    {
        Ok(selected) => selected,
        Err(err) => {
            remote_selection_state
                .release_reserved_target(placement.remote_selection, reserved_node_id.as_deref());
            return Err(err);
        }
    };
    remote_selection_state.confirm_selected_target(
        placement.remote_selection,
        reserved_node_id.as_deref(),
        &selected.node_id,
    );
    placement.ordered_remote_targets = ordered;
    placement.remote_node_id = selected.remote_worker_node_id().map(str::to_string);
    let mut selected = selected;
    selected.excluded_node_ids = failed_node_ids.into_iter().map(str::to_string).collect();
    placement.strict_remote_target = Some(selected);
    Ok(())
}
