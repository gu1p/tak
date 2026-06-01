use std::time::Instant;

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use crate::engine::{
    RemoteWorkspaceStage, StrictRemoteTarget, TaskOutputObserver, TaskStatusEventKind,
    TaskStatusPhase,
};

pub(super) struct UploadProgress {
    started_at: Instant,
}

pub(super) fn start_upload_progress(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    target: &StrictRemoteTarget,
    workspace: &RemoteWorkspaceStage,
) -> Result<UploadProgress> {
    let remote_node_id = target.remote_worker_node_id();
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        remote_node_id,
        format!(
            "upload [----------] 0% {} {}",
            workspace.upload_size_mb(),
            upload_target(target)
        ),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::UploadStart),
            transport: Some(target.transport_kind.as_result_value().to_string()),
            bytes_total: Some(workspace.archive_byte_len),
            bytes_sent: Some(0),
            ..TaskStatusDetails::default()
        },
    )?;
    Ok(UploadProgress {
        started_at: Instant::now(),
    })
}

pub(super) fn finish_upload_progress(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    target: &StrictRemoteTarget,
    workspace: &RemoteWorkspaceStage,
    progress: UploadProgress,
) -> Result<()> {
    let elapsed = progress.started_at.elapsed();
    let elapsed_secs = elapsed.as_secs_f64().max(0.001);
    let mb_per_sec = workspace.archive_byte_len as f64 / 1_000_000.0 / elapsed_secs;
    emit_task_status_message_with_details(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        target.remote_worker_node_id(),
        format!(
            "upload [##########] 100% {} to remote node {} in {:.1}s ({:.2} MB/s)",
            workspace.upload_size_mb(),
            target.node_id,
            elapsed_secs,
            mb_per_sec
        ),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::UploadComplete),
            transport: Some(target.transport_kind.as_result_value().to_string()),
            bytes_total: Some(workspace.archive_byte_len),
            bytes_sent: Some(workspace.archive_byte_len),
            ..TaskStatusDetails::default()
        },
    )
}

fn upload_target(target: &StrictRemoteTarget) -> String {
    if target.is_daemon_tor_placement() {
        "through local takd Tor relay".to_string()
    } else {
        format!("to remote node {}", target.node_id)
    }
}

#[cfg(test)]
mod tests;
