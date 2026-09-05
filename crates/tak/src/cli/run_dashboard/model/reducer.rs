use anyhow::Result;
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use self::event_details::{activity_for, decode_output, task_label};
use super::event_text::{safe_terminal_field, safe_terminal_text};
use super::types::NodeQueueEntry;
use super::{DashboardState, JobActivity, LogLine};

#[path = "reducer/event_details.rs"]
mod event_details;

const LOG_TAIL_LIMIT: usize = 200;

impl DashboardState {
    pub(super) fn apply_event(&mut self, event: &RunEvent) -> Result<()> {
        if self
            .last_applied_event
            .is_some_and(|last| event.seq <= last)
        {
            return Ok(());
        }
        let output = decode_output(event)?;
        self.apply_lifecycle(event);
        self.apply_job(event);
        if let Some(text) = output {
            self.push_log(event, text);
        }
        self.record_failure(event);
        self.last_applied_event = Some(event.seq);
        self.rebuild_nodes();
        Ok(())
    }

    fn apply_lifecycle(&mut self, event: &RunEvent) {
        if event.kind == RunEventKind::Cancelling {
            self.lifecycle = "cancelling".into();
            return;
        }
        if event.job_id.is_some() {
            if matches!(
                event.kind,
                RunEventKind::Transferring | RunEventKind::Running | RunEventKind::OutputCommitting
            ) {
                self.lifecycle = "running".into();
            }
            return;
        }
        let lifecycle = match event.kind {
            RunEventKind::Submitted => "submitted",
            RunEventKind::WorkspaceUploading => "uploading workspace",
            RunEventKind::Queued => "queued",
            RunEventKind::Running => "running",
            RunEventKind::Succeeded => "succeeded",
            RunEventKind::Failed => "failed",
            RunEventKind::Cancelled => "cancelled",
            _ => return,
        };
        self.lifecycle = lifecycle.into();
        if event.kind == RunEventKind::Failed && self.error.is_none() {
            let message = safe_terminal_text(&event.message);
            if !message.is_empty() {
                self.error = Some(message);
            }
        }
    }

    fn apply_job(&mut self, event: &RunEvent) {
        if event.job_id.is_none() && event.kind == RunEventKind::Queued {
            for job in self
                .jobs
                .values_mut()
                .filter(|job| job.activity == JobActivity::Staging)
            {
                job.activity = JobActivity::Ready;
            }
            return;
        }
        if event.job_id.is_none()
            && matches!(
                event.kind,
                RunEventKind::Cancelling | RunEventKind::Cancelled
            )
        {
            for job in self
                .jobs
                .values_mut()
                .filter(|job| !job.activity.is_terminal())
            {
                job.activity = if event.kind == RunEventKind::Cancelling
                    && job.node_id.is_some()
                    && job.activity.is_active()
                {
                    JobActivity::Cancelling
                } else {
                    JobActivity::Cancelled
                };
            }
            return;
        }
        let Some(job) = event.job_id.as_ref().and_then(|id| self.jobs.get_mut(id)) else {
            return;
        };
        if let Some(activity) = activity_for(event.kind) {
            job.activity = activity;
        }
        if event.kind == RunEventKind::Transferring {
            let cache = match event.message.as_str() {
                "workspace cache hit" => Some("hit"),
                "workspace cache miss" => Some("miss"),
                _ => None,
            };
            if let Some(cache) = cache {
                job.cache = Some(cache.into());
            }
        }
        if matches!(event.kind, RunEventKind::Queued | RunEventKind::Retrying) {
            job.node_id = None;
            job.attempt = 0;
            job.cache = None;
        } else if let Some(node_id) = &event.node_id {
            job.node_id = Some(safe_terminal_field(node_id));
        }
        if let Some(attempt) = event.authored_attempt {
            job.attempt = attempt;
        }
    }

    fn push_log(&mut self, event: &RunEvent, text: String) {
        if self.logs.len() == LOG_TAIL_LIMIT {
            self.logs.remove(0);
        }
        self.logs.push(LogLine {
            job: task_label(event),
            node: event
                .node_id
                .as_deref()
                .map(safe_terminal_field)
                .unwrap_or_else(|| "-".into()),
            text,
        });
    }

    fn record_failure(&mut self, event: &RunEvent) {
        if event.kind != RunEventKind::Failed {
            return;
        }
        let message = safe_terminal_text(&event.message);
        if message.is_empty() {
            return;
        }
        if self.error.is_none() {
            self.error = Some(message.clone());
        }
        let task = task_label(event);
        let subject = match (&event.job_id, &event.node_id) {
            (Some(_), Some(node)) => format!("{task}@{}", safe_terminal_field(node)),
            _ => task.clone(),
        };
        self.diagnostics.push(format!("{subject}: {message}"));
        self.push_log(event, format!("failure: {message}"));
    }

    pub(super) fn rebuild_nodes(&mut self) {
        self.nodes.clear();
        for job in self.jobs.values() {
            for node_id in &job.candidate_node_ids {
                self.nodes.entry(node_id.clone()).or_default();
            }
            if let Some(node_id) = job.node_id.as_ref().filter(|_| job.activity.is_active()) {
                self.nodes
                    .entry(node_id.clone())
                    .or_default()
                    .active_jobs
                    .extend(job.task_ids.iter().cloned());
            }
            if job.node_id.is_none() && job.activity == JobActivity::Ready {
                for node_id in &job.candidate_node_ids {
                    let lane = self.nodes.entry(node_id.clone()).or_default();
                    lane.candidate_queue
                        .extend(job.task_ids.iter().cloned().map(|task| NodeQueueEntry {
                            task,
                            queue: job.queue.clone(),
                        }));
                }
            }
        }
    }
}
