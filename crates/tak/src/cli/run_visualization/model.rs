use std::collections::BTreeMap;

use super::reduction::{activity_for, replace_node};
pub(super) use super::state_types::{TaskActivity, TaskRow};
use tak_core::model::TaskLabel;
use tak_exec::{
    PlacementMode, TaskFinishedEvent, TaskStartedEvent, TaskStatusEvent, TaskStatusEventKind,
    TaskStatusPhase, TaskStructuredStatusEvent,
};

pub(super) struct RunState {
    pub(super) jobs: usize,
    pub(super) finished: bool,
    pub(super) error: Option<String>,
    pub(super) rows: BTreeMap<TaskLabel, TaskRow>,
    order: Vec<TaskLabel>,
    member_roots: BTreeMap<TaskLabel, TaskLabel>,
}

impl RunState {
    pub(super) fn new(jobs: usize) -> Self {
        Self {
            jobs: jobs.max(1),
            finished: false,
            error: None,
            rows: BTreeMap::new(),
            order: Vec::new(),
            member_roots: BTreeMap::new(),
        }
    }

    pub(super) fn apply_structured(&mut self, event: TaskStructuredStatusEvent) {
        if event.kind == TaskStatusEventKind::TaskPlanned {
            self.plan(&event);
        }
        let root = self.root_for(&event.task_label);
        let Some(row) = self.rows.get_mut(&root) else {
            return;
        };
        replace_node(row, event.remote_node_id.as_deref());
        if event.transport.is_some() {
            row.transport = event.transport.clone();
        }
        row.attempt = row.attempt.max(event.attempt);
        if matches!(
            event.kind,
            TaskStatusEventKind::QueueAdmission | TaskStatusEventKind::QueuePositionChanged
        ) {
            row.queue_id = event.queue_id.clone();
            row.queue_position = event.queue_position;
        } else {
            row.queue_id = None;
            row.queue_position = None;
        }
        row.activity = activity_for(&event);
        if row.activity.is_terminal() && row.finished_elapsed.is_none() {
            row.finished_elapsed = Some(row.started_at.elapsed());
        }
    }

    pub(super) fn apply_started(&mut self, event: TaskStartedEvent) {
        let root = self.root_for(&event.task_label);
        let Some(row) = self.rows.get_mut(&root) else {
            return;
        };
        row.task_run_id = Some(event.task_run_id);
        match event.placement_mode {
            PlacementMode::Local => {
                row.node = Some("local".into());
                row.activity = TaskActivity::Running;
            }
            PlacementMode::Remote => {
                replace_node(row, event.remote_node_id.as_deref());
                row.activity = TaskActivity::Placing;
            }
        }
    }

    pub(super) fn apply_status(&mut self, event: &TaskStatusEvent) {
        let root = self.root_for(&event.task_label);
        let Some(row) = self.rows.get_mut(&root) else {
            return;
        };
        replace_node(row, event.remote_node_id.as_deref());
        row.attempt = row.attempt.max(event.attempt);
        row.activity = match event.phase {
            TaskStatusPhase::Scheduling => TaskActivity::Waiting,
            TaskStatusPhase::RemoteStageWorkspace => TaskActivity::Staging,
            TaskStatusPhase::RemoteSyncOutputs => TaskActivity::Syncing,
            TaskStatusPhase::RetryWait => TaskActivity::Retrying,
            TaskStatusPhase::RemoteProbe | TaskStatusPhase::RemoteSubmit => TaskActivity::Placing,
            TaskStatusPhase::RemoteWait if row.activity == TaskActivity::Queued => {
                TaskActivity::Queued
            }
            TaskStatusPhase::RemoteWait => TaskActivity::Placing,
        };
    }

    pub(super) fn apply_finished(&mut self, event: TaskFinishedEvent) {
        let root = self.root_for(&event.task_label);
        let Some(row) = self.rows.get_mut(&root) else {
            return;
        };
        row.task_run_id = Some(event.task_run_id);
        row.attempt = event.attempts;
        row.exit_code = event.exit_code;
        replace_node(row, event.remote_node_id.as_deref());
        if event.placement_mode == PlacementMode::Local {
            row.node = Some("local".into());
        }
        row.activity = if row.activity == TaskActivity::Cancelled {
            TaskActivity::Cancelled
        } else if event.success {
            TaskActivity::Passed
        } else {
            TaskActivity::Failed
        };
        row.finished_elapsed = Some(row.started_at.elapsed());
        row.queue_id = None;
        row.queue_position = None;
    }

    pub(super) fn ordered_rows(&self) -> impl Iterator<Item = &TaskRow> {
        self.order.iter().filter_map(|label| self.rows.get(label))
    }

    pub(super) fn total(&self) -> usize {
        self.rows.len()
    }

    pub(super) fn finish(&mut self, error: Option<String>) {
        self.finished = true;
        self.error = error;
    }

    pub(super) fn display_root(&self, label: &TaskLabel) -> TaskLabel {
        self.root_for(label)
    }

    pub(super) fn placement_for(&self, label: &TaskLabel) -> String {
        let root = self.root_for(label);
        self.rows
            .get(&root)
            .and_then(|row| row.node.clone())
            .unwrap_or_else(|| "pending".into())
    }

    pub(super) fn activity_for(&self, label: &TaskLabel) -> Option<TaskActivity> {
        let root = self.root_for(label);
        self.rows.get(&root).map(|row| row.activity)
    }

    fn plan(&mut self, event: &TaskStructuredStatusEvent) {
        let root = event.task_label.clone();
        if !self.rows.contains_key(&root) {
            self.order.push(root.clone());
        }
        let members = if event.execution_unit_members.is_empty() {
            vec![root.clone()]
        } else {
            event.execution_unit_members.clone()
        };
        for member in &members {
            self.member_roots.insert(member.clone(), root.clone());
        }
        self.rows.entry(root.clone()).or_insert(TaskRow {
            label: root,
            member_count: members.len(),
            activity: TaskActivity::Waiting,
            node: None,
            transport: None,
            queue_id: None,
            queue_position: None,
            attempt: 0,
            task_run_id: None,
            exit_code: None,
            started_at: std::time::Instant::now(),
            finished_elapsed: None,
        });
    }

    fn root_for(&self, label: &TaskLabel) -> TaskLabel {
        self.member_roots
            .get(label)
            .cloned()
            .unwrap_or_else(|| label.clone())
    }
}
