use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, anyhow, bail};
use futures::stream::{FuturesUnordered, StreamExt};
use tak_core::model::{ResolvedTask, TaskLabel, WorkspaceSpec};

use super::{LeaseContext, RunOptions, RunSummary, TaskRunResult};

use crate::execution_graph::collect_required_labels;

use super::fused_cascade::{FusedCascade, plan_fused_cascades};
use super::fused_cascade_run::run_fused_cascade;
use super::remote_models::TaskPlacement;
use super::remote_selection::SharedRemoteSelectionState;
use super::run_single_task::run_single_task;
use super::session_cascade::{resolve_cascaded_executions, task_with_execution_override};
use super::session_workspaces::SharedExecutionSessionManager;
use uuid::Uuid;

struct ScheduledUnit {
    root: TaskLabel,
    labels: Vec<TaskLabel>,
    kind: ScheduledUnitKind,
}

enum ScheduledUnitKind {
    Single {
        task: ResolvedTask,
        placement_override: Option<TaskPlacement>,
    },
    Fused(FusedCascade),
}

struct ExecutionPlan {
    units: Vec<ScheduledUnit>,
    dependents: Vec<Vec<usize>>,
    remaining_deps: Vec<usize>,
}

struct ScheduledOutcome {
    unit_id: usize,
    result: Result<TaskRunResult>,
}

/// Executes targets and their transitive dependencies according to DAG order.
///
/// Each task is run with retry policy and optional lease acquisition around attempts.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_tasks(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
    options: &RunOptions,
) -> Result<RunSummary> {
    if targets.is_empty() {
        bail!("at least one target label is required");
    }
    if options.jobs == 0 {
        bail!("jobs must be >= 1");
    }

    let required = collect_required_labels(spec, targets)?;
    let dep_map: BTreeMap<TaskLabel, Vec<TaskLabel>> = required
        .iter()
        .map(|label| {
            let task = spec
                .tasks
                .get(label)
                .ok_or_else(|| anyhow!("missing task for label {label}"))?;
            Ok((label.clone(), task.deps.clone()))
        })
        .collect::<Result<_>>()?;

    let order = tak_core::planner::topo_sort(&dep_map).context("failed to order task execution")?;
    let remote_selection_state = SharedRemoteSelectionState::default();
    let cascaded_executions = resolve_cascaded_executions(
        spec,
        &required,
        &spec.root,
        options.output_observer.as_ref(),
        &remote_selection_state,
    )
    .await?;
    let fused_cascades = plan_fused_cascades(spec, &order, &cascaded_executions)?;
    let plan = build_execution_plan(
        spec,
        &order,
        &dep_map,
        &cascaded_executions,
        &fused_cascades,
    )?;
    let mut summary = RunSummary::default();
    let lease_context = LeaseContext::from_options(options);
    let sessions = SharedExecutionSessionManager::new(Uuid::new_v4().to_string());

    run_execution_plan(
        plan,
        &spec.root,
        options,
        &lease_context,
        &sessions,
        &remote_selection_state,
        &mut summary,
    )
    .await?;

    if summary.results.values().any(|r| !r.success) {
        bail!("one or more tasks failed");
    }

    Ok(summary)
}

fn build_execution_plan(
    spec: &WorkspaceSpec,
    order: &[TaskLabel],
    dep_map: &BTreeMap<TaskLabel, Vec<TaskLabel>>,
    cascaded_executions: &BTreeMap<TaskLabel, super::session_cascade::ExecutionCascadeOverride>,
    fused_cascades: &BTreeMap<TaskLabel, FusedCascade>,
) -> Result<ExecutionPlan> {
    let units = scheduled_units(spec, order, cascaded_executions, fused_cascades)?;
    let label_to_unit = label_to_unit_index(&units);
    let mut dependency_sets = vec![BTreeSet::<usize>::new(); units.len()];
    for (unit_id, unit) in units.iter().enumerate() {
        for label in &unit.labels {
            for dep in dep_map.get(label).into_iter().flatten() {
                let dep_unit = *label_to_unit
                    .get(dep)
                    .ok_or_else(|| anyhow!("missing scheduled dependency for {dep}"))?;
                if dep_unit != unit_id {
                    dependency_sets[unit_id].insert(dep_unit);
                }
            }
        }
    }

    let mut dependents = vec![Vec::new(); units.len()];
    let remaining_deps = dependency_sets
        .iter()
        .map(BTreeSet::len)
        .collect::<Vec<_>>();
    for (unit_id, deps) in dependency_sets.iter().enumerate() {
        for dep in deps {
            dependents[*dep].push(unit_id);
        }
    }

    Ok(ExecutionPlan {
        units,
        dependents,
        remaining_deps,
    })
}

fn scheduled_units(
    spec: &WorkspaceSpec,
    order: &[TaskLabel],
    cascaded_executions: &BTreeMap<TaskLabel, super::session_cascade::ExecutionCascadeOverride>,
    fused_cascades: &BTreeMap<TaskLabel, FusedCascade>,
) -> Result<Vec<ScheduledUnit>> {
    let mut units = Vec::new();
    let mut covered_by_fused_cascade = BTreeSet::new();

    for label in order {
        if covered_by_fused_cascade.contains(label) {
            continue;
        }
        if let Some(fused) = fused_cascades.get(label) {
            let labels = fused
                .members
                .iter()
                .map(|member| member.label.clone())
                .collect::<Vec<_>>();
            for label in &labels {
                covered_by_fused_cascade.insert(label.clone());
            }
            units.push(ScheduledUnit {
                root: fused.root.clone(),
                labels,
                kind: ScheduledUnitKind::Fused(fused.clone()),
            });
            continue;
        }

        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;
        let cascade = cascaded_executions.get(label);
        let effective_task = task_with_execution_override(task, cascade);
        units.push(ScheduledUnit {
            root: label.clone(),
            labels: vec![label.clone()],
            kind: ScheduledUnitKind::Single {
                task: effective_task.unwrap_or_else(|| task.clone()),
                placement_override: cascade.and_then(|cascade| cascade.placement.clone()),
            },
        });
    }

    Ok(units)
}

fn label_to_unit_index(units: &[ScheduledUnit]) -> BTreeMap<TaskLabel, usize> {
    let mut labels = BTreeMap::new();
    for (unit_id, unit) in units.iter().enumerate() {
        for label in &unit.labels {
            labels.insert(label.clone(), unit_id);
        }
    }
    labels
}

async fn run_execution_plan(
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
            Ok(result) => {
                let failed = !result.success;
                insert_unit_result(summary, &plan.units[outcome.unit_id], result.clone());
                if failed && !options.keep_going && terminal_error.is_none() {
                    terminal_error = Some(task_failed_error(&plan.units[outcome.unit_id], &result));
                    ready.clear();
                }
                release_dependents(outcome.unit_id, &plan, &mut remaining_deps, &mut ready);
            }
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
            run_single_task(
                task,
                workspace_root,
                options,
                lease_context,
                sessions,
                remote_selection_state,
                placement_override.clone(),
            )
            .await
        }
        ScheduledUnitKind::Fused(fused) => {
            run_fused_cascade(
                fused,
                workspace_root,
                options,
                lease_context,
                sessions,
                remote_selection_state,
            )
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

fn task_failed_error(unit: &ScheduledUnit, result: &TaskRunResult) -> anyhow::Error {
    if let Some(detail) = result.failure_detail.as_deref() {
        return anyhow!("task {} failed: {detail}", unit.root);
    }
    anyhow!("task {} failed", unit.root)
}
