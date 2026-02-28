use std::collections::BTreeSet;

use anyhow::{Result, anyhow, bail};
use tak_core::model::{TaskLabel, WorkspaceSpec};

/// Collects all tasks required to execute `targets` including transitive dependencies.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn collect_required_labels(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
) -> Result<BTreeSet<TaskLabel>> {
    let mut required = BTreeSet::new();
    let mut visiting = Vec::new();

    for target in targets {
        dfs_collect(target, spec, &mut required, &mut visiting)?;
    }

    Ok(required)
}

/// Depth-first dependency traversal used to populate the required task set.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn dfs_collect(
    label: &TaskLabel,
    spec: &WorkspaceSpec,
    required: &mut BTreeSet<TaskLabel>,
    visiting: &mut Vec<TaskLabel>,
) -> Result<()> {
    if required.contains(label) {
        return Ok(());
    }

    if visiting.contains(label) {
        bail!("cycle detected while collecting dependencies at {label}");
    }

    let task = spec
        .tasks
        .get(label)
        .ok_or_else(|| anyhow!("target does not exist: {label}"))?;

    visiting.push(label.clone());
    for dep in &task.deps {
        dfs_collect(dep, spec, required, visiting)?;
    }
    visiting.pop();

    required.insert(label.clone());
    Ok(())
}
