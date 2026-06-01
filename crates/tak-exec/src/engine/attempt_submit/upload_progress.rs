use std::time::Instant;

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::output_observer::emit_task_status_message;
use crate::engine::{
    RemoteWorkspaceStage, StrictRemoteTarget, TaskOutputObserver, TaskStatusPhase,
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
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(target.node_id.as_str()),
        format!(
            "upload [----------] 0% {} to {}",
            workspace.upload_size_mb(),
            upload_target(target)
        ),
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
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(target.node_id.as_str()),
        format!(
            "upload [##########] 100% {} to remote node {} in {:.1}s ({:.2} MB/s)",
            workspace.upload_size_mb(),
            target.node_id,
            elapsed_secs,
            mb_per_sec
        ),
    )
}

fn upload_target(target: &StrictRemoteTarget) -> String {
    if target.is_daemon_tor_placement() {
        "local takd bridge for Tor relay".to_string()
    } else {
        format!("remote node {}", target.node_id)
    }
}
