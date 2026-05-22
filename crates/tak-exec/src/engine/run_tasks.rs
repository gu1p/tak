use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{TaskLabel, WorkspaceSpec};

use super::{LeaseContext, RunOptions, RunSummary};

use crate::execution_graph::collect_required_labels;

use self::execution_plan::{build_execution_plan, run_execution_plan};
use super::execution_labels::execution_labels_for_targets;
use super::fused_cascade::plan_fused_cascades;
use super::remote_selection::SharedRemoteSelectionState;
use super::session_cascade::resolve_cascaded_executions;
use super::session_workspaces::SharedExecutionSessionManager;
use uuid::Uuid;

mod execution_plan;

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
    let execution_labels = execution_labels_for_targets(spec, targets)?;
    let plan = build_execution_plan(
        spec,
        &order,
        &dep_map,
        &cascaded_executions,
        &fused_cascades,
        &execution_labels,
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
