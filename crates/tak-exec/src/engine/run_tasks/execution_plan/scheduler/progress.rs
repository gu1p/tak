use std::collections::VecDeque;

use anyhow::anyhow;

use super::{ExecutionPlan, ScheduledUnit};
use crate::engine::{RunOptions, RunSummary, TaskRunResult};

pub(super) struct PlanProgress<'a> {
    pub(super) terminal_error: &'a mut Option<anyhow::Error>,
    pub(super) remaining_deps: &'a mut [usize],
    pub(super) ready: &'a mut VecDeque<usize>,
    pub(super) summary: &'a mut RunSummary,
}

pub(super) fn handle_successful_unit(
    unit_id: usize,
    result: TaskRunResult,
    plan: &ExecutionPlan,
    options: &RunOptions,
    progress: PlanProgress<'_>,
) {
    let PlanProgress {
        terminal_error,
        remaining_deps,
        ready,
        summary,
    } = progress;
    let failed = !result.success;
    insert_unit_result(summary, &plan.units[unit_id], result.clone());
    if failed {
        if !options.keep_going && terminal_error.is_none() {
            *terminal_error = Some(task_failed_error(&plan.units[unit_id], &result));
            ready.clear();
        }
        return;
    }
    release_dependents(unit_id, plan, remaining_deps, ready);
}

fn insert_unit_result(summary: &mut RunSummary, unit: &ScheduledUnit, result: TaskRunResult) {
    for label in &unit.labels {
        summary.results.insert(label.clone(), result.clone());
    }
}

fn release_dependents(
    unit_id: usize,
    plan: &ExecutionPlan,
    remaining_deps: &mut [usize],
    ready: &mut VecDeque<usize>,
) {
    for dependent in &plan.dependents[unit_id] {
        remaining_deps[*dependent] -= 1;
        if remaining_deps[*dependent] == 0 {
            ready.push_back(*dependent);
        }
    }
}

fn task_failed_error(unit: &ScheduledUnit, result: &TaskRunResult) -> anyhow::Error {
    if let Some(detail) = result.failure_detail.as_deref() {
        return anyhow!("task {} failed: {detail}", unit.root);
    }
    anyhow!("task {} failed", unit.root)
}
