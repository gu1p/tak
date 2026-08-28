use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use anyhow::Result;

use super::{ExecutionPlan, ScheduledUnit, ScheduledUnitKind};
use crate::engine::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use crate::engine::{TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

pub(super) struct SchedulerStatus<'a> {
    plan: &'a ExecutionPlan,
    observer: Option<&'a Arc<dyn TaskOutputObserver>>,
    positions: BTreeMap<usize, usize>,
    dispatched: BTreeSet<usize>,
    cancelled: BTreeSet<usize>,
}

impl<'a> SchedulerStatus<'a> {
    pub(super) fn new(
        plan: &'a ExecutionPlan,
        observer: Option<&'a Arc<dyn TaskOutputObserver>>,
    ) -> Result<Self> {
        let status = Self {
            plan,
            observer,
            positions: BTreeMap::new(),
            dispatched: BTreeSet::new(),
            cancelled: BTreeSet::new(),
        };
        status.emit_planned_units()?;
        Ok(status)
    }

    pub(super) fn sync_ready(&mut self, ready: &VecDeque<usize>) -> Result<()> {
        let next = visible_positions(self.plan, ready);
        for (unit_id, position) in &next {
            let kind = match self.positions.get(unit_id) {
                None => TaskStatusEventKind::QueueAdmission,
                Some(previous) if previous != position => TaskStatusEventKind::QueuePositionChanged,
                Some(_) => continue,
            };
            self.emit_queue(*unit_id, *position, kind)?;
        }
        self.positions = next;
        Ok(())
    }

    pub(super) fn dispatch(&mut self, unit_id: usize) -> Result<()> {
        self.positions.remove(&unit_id);
        self.dispatched.insert(unit_id);
        let unit = &self.plan.units[unit_id];
        if !is_visible(unit) {
            return Ok(());
        }
        emit_task_status_message_with_details(
            self.observer,
            &unit.root,
            0,
            TaskStatusPhase::Scheduling,
            None,
            "scheduler dispatched task for placement",
            TaskStatusDetails {
                kind: Some(TaskStatusEventKind::Dispatch),
                queue_id: Some("scheduler".into()),
                ..TaskStatusDetails::default()
            },
        )
    }

    pub(super) fn failure(&self, unit_id: usize, message: impl Into<String>) -> Result<()> {
        self.emit_terminal(unit_id, TaskStatusEventKind::FatalFailure, message)
    }

    pub(super) fn cancel_undispatched(&mut self, reason: &str) -> Result<()> {
        for unit_id in 0..self.plan.units.len() {
            if self.dispatched.contains(&unit_id) || self.cancelled.contains(&unit_id) {
                continue;
            }
            if !is_visible(&self.plan.units[unit_id]) {
                continue;
            }
            self.emit_terminal(unit_id, TaskStatusEventKind::Cancellation, reason)?;
            self.cancelled.insert(unit_id);
        }
        self.positions.clear();
        Ok(())
    }

    fn emit_planned_units(&self) -> Result<()> {
        for unit in self.plan.units.iter().filter(|unit| is_visible(unit)) {
            emit_task_status_message_with_details(
                self.observer,
                &unit.root,
                0,
                TaskStatusPhase::Scheduling,
                None,
                "planned execution unit",
                TaskStatusDetails {
                    kind: Some(TaskStatusEventKind::TaskPlanned),
                    execution_unit_members: unit.labels.clone(),
                    ..TaskStatusDetails::default()
                },
            )?;
        }
        Ok(())
    }

    fn emit_queue(&self, unit_id: usize, position: usize, kind: TaskStatusEventKind) -> Result<()> {
        let unit = &self.plan.units[unit_id];
        emit_task_status_message_with_details(
            self.observer,
            &unit.root,
            0,
            TaskStatusPhase::Scheduling,
            None,
            "waiting for a parallel job slot",
            TaskStatusDetails {
                kind: Some(kind),
                queue_id: Some("scheduler".into()),
                queue_position: Some(position),
                ..TaskStatusDetails::default()
            },
        )
    }

    fn emit_terminal(
        &self,
        unit_id: usize,
        kind: TaskStatusEventKind,
        message: impl Into<String>,
    ) -> Result<()> {
        let unit = &self.plan.units[unit_id];
        if !is_visible(unit) {
            return Ok(());
        }
        emit_task_status_message_with_details(
            self.observer,
            &unit.root,
            0,
            TaskStatusPhase::Scheduling,
            None,
            message,
            TaskStatusDetails {
                kind: Some(kind),
                queue_id: Some("scheduler".into()),
                ..TaskStatusDetails::default()
            },
        )
    }
}

fn visible_positions(plan: &ExecutionPlan, ready: &VecDeque<usize>) -> BTreeMap<usize, usize> {
    ready
        .iter()
        .filter(|unit_id| is_visible(&plan.units[**unit_id]))
        .enumerate()
        .map(|(index, unit_id)| (*unit_id, index + 1))
        .collect()
}

fn is_visible(unit: &ScheduledUnit) -> bool {
    match &unit.kind {
        ScheduledUnitKind::Single { task, .. } => !task.steps.is_empty(),
        ScheduledUnitKind::Fused { cascade, .. } => cascade
            .members
            .iter()
            .any(|member| !member.steps.is_empty()),
    }
}
