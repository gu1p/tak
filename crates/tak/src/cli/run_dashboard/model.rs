use std::collections::BTreeMap;

use anyhow::Result;
use tak_proto::local_daemon::v2::{RunEvent, RunLifecycleState};

#[path = "model/event_text.rs"]
mod event_text;
#[path = "model/reducer.rs"]
mod reducer;
#[path = "model/seed.rs"]
mod seed;
#[path = "model/types.rs"]
mod types;

pub(in crate::cli) use types::DashboardSeed;
pub(super) use types::{
    DashboardJob, DashboardJobSeed, DashboardState, JobActivity, LogLine, NodeLane,
};

use self::event_text::safe_terminal_field;

impl DashboardState {
    pub(super) fn new(seed: DashboardSeed) -> Self {
        let mut state = Self {
            run_id: safe_terminal_field(&seed.run_id),
            lifecycle: safe_terminal_field(&seed.lifecycle),
            max_parallel_jobs: seed.max_parallel_jobs,
            jobs: seed
                .jobs
                .into_iter()
                .map(|job| (job.job_id.clone(), DashboardJob::from(job)))
                .collect(),
            nodes: BTreeMap::new(),
            logs: Vec::new(),
            diagnostics: Vec::new(),
            error: None,
            notice: None,
            last_applied_event: None,
        };
        state.rebuild_nodes();
        state
    }

    pub(super) fn apply(&mut self, event: &RunEvent) -> Result<()> {
        self.apply_event(event)
    }

    pub(super) fn sync_lifecycle(&mut self, lifecycle: RunLifecycleState) {
        self.lifecycle = lifecycle.as_str().into();
    }

    pub(super) fn scheduler_queue(&self) -> Vec<&str> {
        self.jobs
            .iter()
            .filter(|(_, job)| {
                job.node_id.is_none()
                    && matches!(job.activity, JobActivity::Ready | JobActivity::Retrying)
            })
            .map(|(job_id, _)| job_id.as_str())
            .collect()
    }

    pub(super) fn terminal_jobs(&self) -> usize {
        self.jobs
            .values()
            .filter(|job| job.activity.is_terminal())
            .count()
    }

    pub(super) fn active_jobs(&self) -> usize {
        self.jobs
            .values()
            .filter(|job| job.activity.is_active() && job.node_id.is_some())
            .count()
    }

    pub(super) fn note_cancellation_persisted(&mut self) {
        self.lifecycle = "cancelling".into();
        self.notice = Some("Cancellation persisted · waiting for takd to stop active work".into());
    }

    pub(super) fn note_already_terminal(&mut self) {
        self.notice = Some("Run was already terminal · loading its final state".into());
    }

    pub(super) fn note_input_lost(&mut self) {
        self.notice = Some("Keyboard navigation unavailable · Ctrl-C remains available".into());
    }

    pub(super) fn note_logs_expired(&mut self) {
        self.notice = Some("Run logs have expired.".into());
    }
}
