use anyhow::Result;
use tak_core::model::TaskLabel;

use super::{StrictRemoteTarget, TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

use super::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};

pub(crate) fn emit_remote_probe(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    if StrictRemoteTarget::is_daemon_tor_node_id(node_id) {
        return emit_task_status_message_with_details(
            output_observer,
            task_label,
            attempt,
            TaskStatusPhase::RemoteProbe,
            None,
            "connecting to local takd daemon",
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::LocalDaemonConnection),
                local_daemon_path: Some(
                    super::transport::broker_socket_path().display().to_string(),
                ),
                transport: Some("tor".to_string()),
                ..TaskStatusDetails::default()
            },
        );
    }
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("probing remote node {node_id}"),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::RemoteNodeProbe),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn emit_remote_connected(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    if StrictRemoteTarget::is_daemon_tor_node_id(node_id) {
        return emit_task_status_message_with_details(
            output_observer,
            task_label,
            attempt,
            TaskStatusPhase::RemoteProbe,
            None,
            "local takd: discovering remote capacity over Tor",
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::RemoteCapacityDiscovery),
                local_daemon_path: Some(
                    super::transport::broker_socket_path().display().to_string(),
                ),
                transport: Some("tor".to_string()),
                ..TaskStatusDetails::default()
            },
        );
    }
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("connected to remote node {node_id}"),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::RemoteNodeConnected),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn emit_remote_unavailable(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    if StrictRemoteTarget::is_daemon_tor_node_id(node_id) {
        return emit_task_status_message_with_details(
            output_observer,
            task_label,
            attempt,
            TaskStatusPhase::RemoteProbe,
            None,
            "local takd Tor relay unavailable, trying next candidate",
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::RemoteNodeUnavailable),
                local_daemon_path: Some(
                    super::transport::broker_socket_path().display().to_string(),
                ),
                transport: Some("tor".to_string()),
                ..TaskStatusDetails::default()
            },
        );
    }
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("remote node {node_id} unavailable, trying next candidate"),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::RemoteNodeUnavailable),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn emit_remote_submit(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
    upload_size_mb: &str,
) -> Result<()> {
    if StrictRemoteTarget::is_daemon_tor_node_id(node_id) {
        return emit_task_status_message_with_details(
            output_observer,
            task_label,
            attempt,
            TaskStatusPhase::RemoteSubmit,
            None,
            format!("submitting {upload_size_mb} through local takd Tor relay"),
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::Dispatch),
                local_daemon_path: Some(
                    super::transport::broker_socket_path().display().to_string(),
                ),
                transport: Some("tor".to_string()),
                ..TaskStatusDetails::default()
            },
        );
    }
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(node_id),
        format!("submitting {upload_size_mb} to remote node {node_id}"),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::Dispatch),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn emit_remote_accepted(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(node_id),
        format!("remote task accepted by {node_id}"),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::WorkerSelected),
            transport: Some("tor".to_string()),
            ..TaskStatusDetails::default()
        },
    )
}

pub(crate) fn next_candidate_available(
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    index: usize,
) -> bool {
    candidates[index + 1..]
        .iter()
        .any(|next| next.node_id != failed_node_id)
}

#[cfg(test)]
mod connected_tests;
#[cfg(test)]
mod tests;
