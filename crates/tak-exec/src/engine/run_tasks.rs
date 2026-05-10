use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{TaskLabel, WorkspaceSpec};

use super::{LeaseContext, RunOptions, RunSummary};

use crate::execution_graph::collect_required_labels;

use super::fused_cascade::plan_fused_cascades;
use super::fused_cascade_run::run_fused_cascade;
use super::run_single_task::run_single_task;
use super::session_cascade::{resolve_cascaded_executions, task_with_execution_override};
use super::session_workspaces::ExecutionSessionManager;
use uuid::Uuid;

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
    let cascaded_executions = resolve_cascaded_executions(
        spec,
        &required,
        &spec.root,
        options.output_observer.as_ref(),
    )
    .await?;
    let fused_cascades = plan_fused_cascades(spec, &order, &cascaded_executions)?;
    let mut summary = RunSummary::default();
    let lease_context = LeaseContext::from_options(options);
    let mut sessions = ExecutionSessionManager::new(Uuid::new_v4().to_string());
    let mut covered_by_fused_cascade = BTreeSet::new();

    for label in order {
        if covered_by_fused_cascade.contains(&label) {
            continue;
        }
        if let Some(fused) = fused_cascades.get(&label) {
            let task_result =
                run_fused_cascade(fused, &spec.root, options, &lease_context, &mut sessions)
                    .await?;
            let failed = !task_result.success;
            for member in &fused.members {
                summary
                    .results
                    .insert(member.label.clone(), task_result.clone());
                covered_by_fused_cascade.insert(member.label.clone());
            }

            if failed && !options.keep_going {
                if let Some(detail) = task_result.failure_detail.as_deref() {
                    bail!("task {} failed: {detail}", fused.root);
                }
                bail!("task {} failed", fused.root);
            }
            continue;
        }

        let task = spec
            .tasks
            .get(&label)
            .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;
        let cascade = cascaded_executions.get(&label);
        let effective_task = task_with_execution_override(task, cascade);
        let task = effective_task.as_ref().unwrap_or(task);
        let placement_override = cascade.and_then(|cascade| cascade.placement.clone());

        let task_result = run_single_task(
            task,
            &spec.root,
            options,
            &lease_context,
            &mut sessions,
            placement_override,
        )
        .await?;
        let failed = !task_result.success;
        summary.results.insert(label.clone(), task_result);

        if failed && !options.keep_going {
            let failure_detail = summary
                .results
                .get(&label)
                .and_then(|result| result.failure_detail.as_deref());
            if let Some(detail) = failure_detail {
                bail!("task {label} failed: {detail}");
            }
            bail!("task {label} failed");
        }
    }

    if summary.results.values().any(|r| !r.success) {
        bail!("one or more tasks failed");
    }

    Ok(summary)
}
