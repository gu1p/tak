use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tak_core::model::TaskLabel;

use crate::engine::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use crate::engine::{TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

pub(crate) struct StreamUploadProgress<'a> {
    pub(crate) observer: Option<&'a Arc<dyn TaskOutputObserver>>,
    pub(crate) task_label: &'a TaskLabel,
    pub(crate) attempt: u32,
}

pub(super) struct ActiveStreamUploadProgress<'a> {
    input: StreamUploadProgress<'a>,
    total: u64,
    last_reported: u64,
    started_at: Instant,
}

impl<'a> ActiveStreamUploadProgress<'a> {
    pub(super) fn new(input: StreamUploadProgress<'a>, total: u64) -> Self {
        Self {
            input,
            total,
            last_reported: 0,
            started_at: Instant::now(),
        }
    }

    pub(super) fn report(&mut self, peer_node_id: &str, sent: u64, force: bool) -> Result<()> {
        let step = (self.total / 20).max(1024 * 1024);
        if !force && sent.saturating_sub(self.last_reported) < step {
            return Ok(());
        }
        self.last_reported = sent;
        let elapsed = self.started_at.elapsed().as_secs_f64().max(0.001);
        let mb_sent = sent as f64 / 1_000_000.0;
        let mb_total = self.total as f64 / 1_000_000.0;
        let pct = if self.total == 0 {
            100.0
        } else {
            sent as f64 * 100.0 / self.total as f64
        };
        emit_task_status_message_with_details(
            self.input.observer,
            self.input.task_label,
            self.input.attempt,
            TaskStatusPhase::RemoteSubmit,
            Some(peer_node_id),
            format!(
                "upload {:.0}% {:.2}/{:.2} MB to remote node {} ({:.2} MB/s)",
                pct,
                mb_sent,
                mb_total,
                peer_node_id,
                mb_sent / elapsed
            ),
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::UploadProgress),
                transport: Some("tor".to_string()),
                bytes_total: Some(self.total),
                bytes_sent: Some(sent),
                ..TaskStatusDetails::default()
            },
        )
    }
}
