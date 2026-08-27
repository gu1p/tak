use anyhow::{Result, anyhow};
use tak_core::model::TaskLabel;

use super::output_observer::{
    TaskStatusDetails, emit_task_status_message, emit_task_status_message_with_details,
};
use super::preflight_status_output::emit_remote_probe;
use super::remote_failure::RemoteFailureKind;
use super::remote_models::{RemoteInfrastructureFailure, StrictRemoteTarget, TaskPlacement};
use super::remote_selection::SharedRemoteSelectionState;
use super::{TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

pub(crate) fn prepare_remote_failover(
    placement: &mut TaskPlacement,
    cause: String,
    failure_kind: Option<RemoteFailureKind>,
    selection_state: &SharedRemoteSelectionState,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
) -> Result<()> {
    let node_id = placement
        .remote_node_id
        .clone()
        .or_else(|| {
            placement
                .strict_remote_target
                .as_ref()
                .map(|target| target.node_id.clone())
        })
        .ok_or_else(|| exhausted_diagnostic(placement, Some(&cause)))?;
    if StrictRemoteTarget::is_daemon_tor_node_id(&node_id) {
        return Err(exhausted_diagnostic(placement, Some(&cause)));
    }
    selection_state.release_reserved_target(placement.remote_selection, Some(&node_id));
    if !placement
        .infrastructure_failures
        .iter()
        .any(|failure| failure.node_id == node_id)
    {
        placement
            .infrastructure_failures
            .push(RemoteInfrastructureFailure {
                node_id: node_id.clone(),
                cause,
            });
    }
    let excluded = placement
        .infrastructure_failures
        .iter()
        .map(|failure| failure.node_id.clone())
        .collect::<Vec<_>>();
    let remaining_direct = placement
        .ordered_remote_targets
        .iter()
        .any(|target| !target.is_daemon_tor_placement() && !excluded.contains(&target.node_id));
    if remaining_direct {
        placement.remote_node_id = None;
        placement.strict_remote_target = None;
    } else if let Some(remote) = placement.remote.as_ref()
        && (placement.ordered_remote_targets.is_empty()
            || placement
                .ordered_remote_targets
                .iter()
                .any(StrictRemoteTarget::is_daemon_tor_placement))
    {
        let mut target = StrictRemoteTarget::daemon_tor_placement(remote);
        target.excluded_node_ids = excluded;
        placement.remote_node_id = None;
        placement.strict_remote_target = Some(target);
    } else {
        return Err(exhausted_diagnostic(placement, None));
    }
    emit_remote_retry_status(
        output_observer,
        task_label,
        attempt,
        &node_id,
        failure_kind,
        placement.strict_remote_target.as_ref(),
    )
}

fn emit_remote_retry_status(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    failed_node_id: &str,
    failure_kind: Option<RemoteFailureKind>,
    replacement: Option<&StrictRemoteTarget>,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RetryWait,
        Some(failed_node_id),
        retry_failure_message(failure_kind, failed_node_id),
    )?;
    let Some(replacement) = replacement.filter(|target| target.is_daemon_tor_placement()) else {
        return Ok(());
    };
    emit_remote_probe(output_observer, task_label, attempt, &replacement.node_id)?;
    emit_replacement_connection_status(output_observer, task_label, attempt, failed_node_id)
}

pub(super) fn retry_failure_message(
    failure_kind: Option<RemoteFailureKind>,
    failed_node_id: &str,
) -> String {
    match failure_kind {
        Some(RemoteFailureKind::ResourceCapacity) => format!(
            "remote resource-capacity stop on {failed_node_id}; retrying on another eligible worker"
        ),
        _ => format!(
            "remote infrastructure failure on {failed_node_id}; retrying on another eligible worker"
        ),
    }
}

fn emit_replacement_connection_status(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    failed_node_id: &str,
) -> Result<()> {
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        None,
        format!(
            "local takd: connecting to a replacement worker over Tor (excluding failed worker {failed_node_id})"
        ),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::RemoteCapacityDiscovery),
            local_daemon_path: Some(super::transport::broker_socket_path().display().to_string()),
            transport: Some("tor".to_string()),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn exhausted_diagnostic(
    placement: &TaskPlacement,
    final_cause: Option<&str>,
) -> anyhow::Error {
    let mut lines = vec!["remote worker failover exhausted all eligible workers".to_string()];
    for failure in &placement.infrastructure_failures {
        lines.push(format!("- {}: {}", failure.node_id, failure.cause));
    }
    if let Some(cause) = final_cause {
        lines.push(format!("- replacement placement: {cause}"));
    }
    anyhow!(lines.join("\n"))
}
