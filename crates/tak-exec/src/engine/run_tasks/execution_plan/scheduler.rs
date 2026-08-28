use std::collections::VecDeque;

use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};

use super::{ExecutionPlan, ScheduledUnit, ScheduledUnitKind};
use crate::engine::fused_cascade_run::{FusedCascadeRunContext, run_fused_cascade};
use crate::engine::remote_selection::SharedRemoteSelectionState;
use crate::engine::run_single_task::{RunSingleTaskContext, run_single_task};
use crate::engine::session_workspaces::SharedExecutionSessionManager;
use crate::engine::{LeaseContext, RunOptions, RunSummary, TaskRunResult};

mod progress;
mod status;
use progress::{PlanProgress, handle_successful_unit};
use status::SchedulerStatus;

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
    let mut status = SchedulerStatus::new(&plan, options.output_observer.as_ref())?;
    status.sync_ready(&ready)?;

    while completed < plan.units.len() {
        while terminal_error.is_none()
            && running.len() < options.jobs
            && let Some(unit_id) = ready.pop_front()
        {
            status.dispatch(unit_id)?;
            status.sync_ready(&ready)?;
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
            Ok(result) => {
                if !result.success {
                    status.failure(outcome.unit_id, "scheduled execution unit failed")?;
                }
                handle_successful_unit(
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
                )
            }
            Err(err) => {
                status.failure(
                    outcome.unit_id,
                    format!("scheduled execution failed: {err:#}"),
                )?;
                if terminal_error.is_none() {
                    terminal_error = Some(err);
                    ready.clear();
                }
            }
        }
        status.sync_ready(&ready)?;
        if terminal_error.is_some() {
            status.cancel_undispatched("cancelled by fail-fast scheduling")?;
        }
    }

    if completed < plan.units.len() {
        status.cancel_undispatched("cancelled because a dependency did not succeed")?;
    }

    if let Some(err) = terminal_error {
        return Err(err);
    }
    Ok(())
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
