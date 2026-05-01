use anyhow::{Result, bail};
use tak_core::model::{RemoteSpec, ResolvedTask};

use super::placement::PlacementCandidate;
use super::remote_models::TaskPlacement;
use super::{NoMatchingRemoteError, PlacementMode};
use crate::client_remotes::configured_remote_targets;

pub(crate) fn remote_task_candidate(
    task: &ResolvedTask,
    remote: &RemoteSpec,
    reason: Option<String>,
) -> Result<PlacementCandidate> {
    let remote = materialize_effective_remote_spec(task, remote)?;
    let selection = configured_remote_targets(&remote)?;
    if selection.matched_targets.is_empty() {
        return Ok(PlacementCandidate::Unavailable(
            NoMatchingRemoteError::new(
                canonical_task_label(&task.label),
                &remote,
                selection.configured_remote_count,
                selection.enabled_remote_count,
                selection.enabled_remotes,
            )
            .into(),
        ));
    }
    Ok(PlacementCandidate::Ready(Box::new(TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: selection.matched_targets,
        remote_selection: remote.selection,
        decision_reason: reason,
        session: remote.session.clone(),
        local: None,
        remote: Some(remote.clone()),
    })))
}

fn materialize_effective_remote_spec(
    task: &ResolvedTask,
    remote: &RemoteSpec,
) -> Result<RemoteSpec> {
    if remote.runtime.is_some() {
        return Ok(remote.clone());
    }
    if let Some(runtime) = task.container_runtime.clone() {
        let mut remote = remote.clone();
        remote.runtime = Some(runtime);
        return Ok(remote);
    }

    bail!(
        "task {} requires a container for remote execution; provide Execution.Remote(..., container=Container.Image(...)), Decision.remote(..., container=Container.Image(...)), or TASKS.py defaults.container",
        canonical_task_label(&task.label)
    )
}

fn canonical_task_label(label: &tak_core::model::TaskLabel) -> String {
    format!("{}:{}", label.package, label.name)
}
