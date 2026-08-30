use std::io::Write;

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

pub(super) fn event(event: &RunEvent) -> Result<()> {
    if let Some(chunk) = &event.chunk_base64 {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(chunk)
            .context("local takd returned an invalid output event")?;
        match event.kind {
            RunEventKind::Stdout => write(&mut std::io::stdout().lock(), &bytes)?,
            RunEventKind::Stderr => write(&mut std::io::stderr().lock(), &bytes)?,
            _ => bail!("local takd returned an invalid output event"),
        }
        return Ok(());
    }
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
    Ok(())
}

fn write(stream: &mut impl Write, bytes: &[u8]) -> Result<()> {
    stream.write_all(bytes)?;
    stream.flush()?;
    Ok(())
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
        RunEventKind::Stdout => "stdout",
        RunEventKind::Stderr => "stderr",
    }
}
