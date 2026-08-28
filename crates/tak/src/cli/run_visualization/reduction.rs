use tak_exec::{TaskStatusEventKind, TaskStatusPhase, TaskStructuredStatusEvent};

use super::state_types::{TaskActivity, TaskRow};

pub(super) fn activity_for(event: &TaskStructuredStatusEvent) -> TaskActivity {
    use TaskStatusEventKind as Kind;
    match event.kind {
        Kind::TaskPlanned => TaskActivity::Waiting,
        Kind::QueueAdmission | Kind::QueuePositionChanged => {
            if event.queue_id.as_deref() == Some("scheduler") {
                TaskActivity::Waiting
            } else {
                TaskActivity::Queued
            }
        }
        Kind::WorkspaceStage => TaskActivity::Staging,
        Kind::UploadStart | Kind::UploadProgress | Kind::UploadComplete => TaskActivity::Uploading,
        Kind::RemoteExecutionStart => TaskActivity::Running,
        Kind::RetryScheduled => TaskActivity::Retrying,
        Kind::FatalFailure => TaskActivity::Failed,
        Kind::Cancellation => TaskActivity::Cancelled,
        Kind::Completion => TaskActivity::Passed,
        _ if event.phase == TaskStatusPhase::RemoteSyncOutputs => TaskActivity::Syncing,
        _ => TaskActivity::Placing,
    }
}

pub(super) fn replace_node(row: &mut TaskRow, candidate: Option<&str>) {
    if let Some(node) = candidate.filter(|node| !node.is_empty() && !node.starts_with("__takd_")) {
        row.node = Some(node.to_string());
    }
}
