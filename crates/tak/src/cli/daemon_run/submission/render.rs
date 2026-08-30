use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

pub(super) fn event(event: &RunEvent) {
    let tasks = if event.task_ids.is_empty() {
        "-".to_owned()
    } else {
        event.task_ids.join(",")
    };
    println!(
        "{} tasks={} node={} {}",
        event_kind(event.kind),
        tasks,
        event.node_id.as_deref().unwrap_or("-"),
        event.message
    );
}

fn event_kind(kind: RunEventKind) -> &'static str {
    match kind {
        RunEventKind::Submitted => "submitted",
        RunEventKind::WorkspaceUploading => "workspace_uploading",
        RunEventKind::Queued => "queued",
        RunEventKind::Transferring => "transferring",
        RunEventKind::Running => "running",
        RunEventKind::Retrying => "retrying",
        RunEventKind::OutputCommitting => "output_committing",
        RunEventKind::Cancelling => "cancelling",
        RunEventKind::Succeeded => "succeeded",
        RunEventKind::Failed => "failed",
        RunEventKind::Cancelled => "cancelled",
        RunEventKind::Skipped => "skipped",
    }
}
