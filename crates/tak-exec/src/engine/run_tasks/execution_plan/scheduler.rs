use std::collections::VecDeque;

use anyhow::{Result, anyhow};
use futures::stream::{FuturesUnordered, StreamExt};

use super::{ExecutionPlan, ScheduledUnit, ScheduledUnitKind};
use crate::engine::fused_cascade_run::{FusedCascadeRunContext, run_fused_cascade};
use crate::engine::remote_selection::SharedRemoteSelectionState;
use crate::engine::run_single_task::{RunSingleTaskContext, run_single_task};
use crate::engine::session_workspaces::SharedExecutionSessionManager;
use crate::engine::{LeaseContext, RunOptions, RunSummary, TaskRunResult};

struct ScheduledOutcome {
    unit_id: usize,
    result: Result<TaskRunResult>,
}

pub(in crate::engine::run_tasks) async fn run_execution_plan(
    plan: ExecutionPlan,
    workspace_root: &std::path::Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &SharedExecutionSessionManager,
    remote_selection_state: &SharedRemoteSelectionState,
    summary: &mut RunSummary,
) -> Result<()> {
    let mut remaining_deps = plan.remaining_deps.clone();
    let mut ready = ready_units(&remaining_deps);
    let mut running = FuturesUnordered::new();
    let mut completed = 0_usize;
    let mut terminal_error = None;

    while completed < plan.units.len() {
        while terminal_error.is_none()
            && running.len() < options.jobs
            && let Some(unit_id) = ready.pop_front()
        {
            running.push(run_scheduled_unit(
                unit_id,
                &plan.units[unit_id],
                workspace_root,
                options,
                lease_context,
                sessions,
                remote_selection_state,
            ));
        }

        let Some(outcome) = running.next().await else {
            break;
        };
        completed += 1;
        match outcome.result {
            Ok(result) => handle_successful_unit(
                outcome.unit_id,
                result,
                &plan,
                options,
                PlanProgress {
                    terminal_error: &mut terminal_error,
                    remaining_deps: &mut remaining_deps,
                    ready: &mut ready,
                    summary,
                },
            ),
            Err(err) => {
                if terminal_error.is_none() {
                    terminal_error = Some(err);
                    ready.clear();
                }
            }
        }
    }

    if let Some(err) = terminal_error {
        return Err(err);
    }
    Ok(())
}

fn handle_successful_unit(
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
    if failed && !options.keep_going && terminal_error.is_none() {
        *terminal_error = Some(task_failed_error(&plan.units[unit_id], &result));
        ready.clear();
    }
    release_dependents(unit_id, plan, remaining_deps, ready);
}

fn ready_units(remaining_deps: &[usize]) -> VecDeque<usize> {
    remaining_deps
        .iter()
        .enumerate()
        .filter_map(|(unit_id, count)| (*count == 0).then_some(unit_id))
        .collect()
}

async fn run_scheduled_unit(
    unit_id: usize,
    unit: &ScheduledUnit,
    workspace_root: &std::path::Path,
    options: &RunOptions,
    lease_context: &LeaseContext,
    sessions: &SharedExecutionSessionManager,
    remote_selection_state: &SharedRemoteSelectionState,
) -> ScheduledOutcome {
    let result = match &unit.kind {
        ScheduledUnitKind::Single {
            task,
            placement_override,
        } => {
            run_single_task(RunSingleTaskContext {
                task,
                workspace_root,
                options,
                lease_context,
                sessions,
                remote_selection_state,
                execution_label: Some(unit.execution_label.as_str()),
                placement_override: placement_override.clone(),
            })
            .await
        }
        ScheduledUnitKind::Fused {
            cascade,
            member_execution_labels,
        } => {
            run_fused_cascade(FusedCascadeRunContext {
                cascade,
                workspace_root,
                options,
                lease_context,
                sessions,
                remote_selection_state,
                execution_label: Some(unit.execution_label.as_str()),
                member_execution_labels,
            })
            .await
        }
    };
    ScheduledOutcome { unit_id, result }
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

struct PlanProgress<'a> {
    terminal_error: &'a mut Option<anyhow::Error>,
    remaining_deps: &'a mut [usize],
    ready: &'a mut VecDeque<usize>,
    summary: &'a mut RunSummary,
}

fn task_failed_error(unit: &ScheduledUnit, result: &TaskRunResult) -> anyhow::Error {
    if let Some(detail) = result.failure_detail.as_deref() {
        return anyhow!("task {} failed: {detail}", unit.root);
    }
    anyhow::anyhow!("task {} failed", unit.root)
}
