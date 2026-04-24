use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{TaskLabel, WorkspaceSpec};

use super::{LeaseContext, RunOptions, RunSummary};

use crate::execution_graph::collect_required_labels;

use super::run_single_task::run_single_task;
use super::session_cascade::{resolve_cascaded_sessions, task_with_session_override};
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
    let cascaded_sessions = resolve_cascaded_sessions(spec, &required)?;
    let mut summary = RunSummary::default();
    let lease_context = LeaseContext::from_options(options);
    let mut sessions = ExecutionSessionManager::new(Uuid::new_v4().to_string());

    for label in order {
        let task = spec
            .tasks
            .get(&label)
            .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;
        let effective_task = task_with_session_override(task, cascaded_sessions.get(&label));
        let task = effective_task.as_ref().unwrap_or(task);

        let prepared_session = if task.steps.is_empty() {
            None
        } else {
            sessions.prepare_task(task, &spec.root)?
        };
        let task_result = run_single_task(
            task,
            &spec.root,
            options,
            &lease_context,
            prepared_session.as_ref(),
        )
        .await?;
        sessions.finish_task(prepared_session.as_ref(), task_result.success)?;
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
