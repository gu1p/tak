use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::super::JobActivity;
use super::super::event_text::{safe_terminal_field, safe_terminal_text};

pub(super) fn task_label(event: &RunEvent) -> String {
    if event.task_ids.is_empty() {
        return event
            .job_id
            .as_deref()
            .map(safe_terminal_field)
            .unwrap_or_else(|| "run".into());
    }
    event
        .task_ids
        .iter()
        .map(|task| safe_terminal_field(task))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn decode_output(event: &RunEvent) -> Result<Option<String>> {
    let Some(encoded) = &event.chunk_base64 else {
        return Ok(None);
    };
    let bytes = STANDARD
        .decode(encoded)
        .context("local takd returned invalid dashboard output")?;
    Ok(Some(safe_terminal_text(&String::from_utf8_lossy(&bytes))))
}

pub(super) fn activity_for(kind: RunEventKind) -> Option<JobActivity> {
    match kind {
        RunEventKind::Queued => Some(JobActivity::Ready),
        RunEventKind::Transferring | RunEventKind::WorkspaceUploading => {
            Some(JobActivity::Transferring)
        }
        RunEventKind::Running => Some(JobActivity::Running),
        RunEventKind::Retrying => Some(JobActivity::Retrying),
        RunEventKind::OutputCommitting => Some(JobActivity::OutputCommitting),
        RunEventKind::Cancelling => Some(JobActivity::Cancelling),
        RunEventKind::Succeeded => Some(JobActivity::Succeeded),
        RunEventKind::Failed => Some(JobActivity::Failed),
        RunEventKind::Cancelled => Some(JobActivity::Cancelled),
        RunEventKind::Skipped => Some(JobActivity::Skipped),
        RunEventKind::Submitted | RunEventKind::Stdout | RunEventKind::Stderr => None,
    }
}
