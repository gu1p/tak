use anyhow::{Result, bail};
use tak_core::model::{RemoteSpec, RemoteTransportKind, ResolvedTask};

use super::placement::PlacementCandidate;
use super::remote_models::{StrictRemoteTarget, TaskPlacement};
use super::{NoMatchingRemoteError, PlacementMode};
use crate::client_remotes::configured_remote_targets;

pub(crate) fn remote_task_candidate(
    task: &ResolvedTask,
    remote: &RemoteSpec,
    reason: Option<String>,
) -> Result<PlacementCandidate> {
    let remote = materialize_effective_remote_spec(task, remote)?;
    if remote.transport_kind == RemoteTransportKind::Tor {
        return Ok(PlacementCandidate::Ready(Box::new(daemon_tor_placement(
            &remote, reason,
        ))));
    }
    let selection = match configured_remote_targets(&remote) {
        Ok(selection) => selection,
        Err(_err) if remote.transport_kind == RemoteTransportKind::Any => {
            return Ok(PlacementCandidate::Ready(Box::new(daemon_tor_placement(
                &remote, reason,
            ))));
        }
        Err(err) => return Err(err),
    };
    if selection.matched_targets.is_empty()
        && (remote.transport_kind != RemoteTransportKind::Any
            || selection.matched_tor_target_count == 0)
    {
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
    let matched_targets = if remote.transport_kind == RemoteTransportKind::Any {
        direct_targets_with_daemon_tor_fallback(
            selection.matched_targets,
            &remote,
            selection.matched_tor_target_count,
        )
    } else {
        selection.matched_targets
    };
    Ok(PlacementCandidate::Ready(Box::new(TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: matched_targets,
        remote_selection: remote.selection,
        decision_reason: reason,
        session: remote.session.clone(),
        local: None,
        remote: Some(remote.clone()),
    })))
}

fn daemon_tor_placement(remote: &RemoteSpec, reason: Option<String>) -> TaskPlacement {
    TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: None,
        strict_remote_target: Some(StrictRemoteTarget::daemon_tor_placement(remote)),
        ordered_remote_targets: Vec::new(),
        remote_selection: remote.selection,
        decision_reason: reason,
        session: remote.session.clone(),
        local: None,
        remote: Some(remote.clone()),
    }
}

fn direct_targets_with_daemon_tor_fallback(
    mut targets: Vec<StrictRemoteTarget>,
    remote: &RemoteSpec,
    matched_tor_target_count: usize,
) -> Vec<StrictRemoteTarget> {
    if matched_tor_target_count > 0 {
        targets.push(StrictRemoteTarget::daemon_tor_placement(remote));
    }
    targets
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
