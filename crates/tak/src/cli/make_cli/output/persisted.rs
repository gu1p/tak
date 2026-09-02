use anyhow::{Result, bail};
use base64::Engine as _;
use tak_core::label::parse_label;
use tak_exec::{OutputStream, PlacementMode, TaskFinishedEvent, TaskOutputChunk};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::ParallelMakeOutputObserver;

impl crate::cli::daemon_run::PersistedEventRenderer for ParallelMakeOutputObserver {
    fn render(&self, event: &RunEvent) -> Result<bool> {
        if let Some(chunk) = &event.chunk_base64 {
            let task_id = event
                .task_ids
                .first()
                .ok_or_else(|| anyhow::anyhow!("Make output event has no task"))?;
            let stream = match event.kind {
                RunEventKind::Stdout => OutputStream::Stdout,
                RunEventKind::Stderr => OutputStream::Stderr,
                _ => bail!("Make output event has an invalid kind"),
            };
            self.observe_lines(TaskOutputChunk {
                task_run_id: event.job_id.clone().unwrap_or_default(),
                task_label: parse_label(task_id, "//")?,
                attempt: 1,
                stream,
                bytes: base64::engine::general_purpose::STANDARD.decode(chunk)?,
            })?;
            return Ok(true);
        }
        if matches!(event.kind, RunEventKind::Succeeded | RunEventKind::Failed) {
            for task_id in &event.task_ids {
                self.finish(TaskFinishedEvent {
                    task_run_id: event.job_id.clone().unwrap_or_default(),
                    task_label: parse_label(task_id, "//")?,
                    attempts: 1,
                    success: event.kind == RunEventKind::Succeeded,
                    exit_code: event.exit_code,
                    placement_mode: placement(event),
                    remote_node_id: remote_node(event),
                })?;
            }
        }
        Ok(false)
    }
}

fn placement(event: &RunEvent) -> PlacementMode {
    if event.node_id.as_deref() == Some("local") {
        PlacementMode::Local
    } else {
        PlacementMode::Remote
    }
}

fn remote_node(event: &RunEvent) -> Option<String> {
    (event.node_id.as_deref() != Some("local"))
        .then(|| event.node_id.clone())
        .flatten()
}
