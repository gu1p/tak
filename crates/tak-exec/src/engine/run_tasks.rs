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
    let mut summary = RunSummary::default();
    let lease_context = LeaseContext::from_options(options);

    for label in order {
        let task = spec
            .tasks
            .get(&label)
            .ok_or_else(|| anyhow!("missing task definition for label {label}"))?;

        let task_result = run_single_task(task, &spec.root, options, &lease_context).await?;
        let failed = !task_result.success;
        summary.results.insert(label.clone(), task_result);

        if failed && !options.keep_going {
            bail!("task {label} failed");
        }
    }

    if summary.results.values().any(|r| !r.success) {
        bail!("one or more tasks failed");
    }

    Ok(summary)
}
