use std::collections::BTreeSet;
use tak_core::model::{ResolvedTask, TaskExecutionSpec, TaskLabel, WorkspaceSpec};

use super::*;

pub(super) fn sessions_used_by_closure(
    spec: &WorkspaceSpec,
    closure: &BTreeSet<TaskLabel>,
) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for label in closure {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow!("task not found: {}", canonical_label(label)))?;
        if let TaskExecutionSpec::UseSession { name, .. } = &task.execution {
            names.insert(name.clone());
        }
    }
    Ok(names)
}

pub(super) fn first_task_using_session<'a>(
    spec: &'a WorkspaceSpec,
    closure: &BTreeSet<TaskLabel>,
    session_name: &str,
) -> Result<&'a ResolvedTask> {
    for label in closure {
        let task = spec
            .tasks
            .get(label)
            .ok_or_else(|| anyhow!("task not found: {}", canonical_label(label)))?;
        if matches!(&task.execution, TaskExecutionSpec::UseSession { name, .. } if name == session_name)
        {
            return Ok(task);
        }
    }
    bail!("session not used by selected target closure: {session_name}")
}

pub(super) fn target_closure(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
) -> Result<BTreeSet<TaskLabel>> {
    let mut closure = BTreeSet::new();
    let mut stack = targets.to_vec();
    while let Some(label) = stack.pop() {
        if !closure.insert(label.clone()) {
            continue;
        }
        let task = spec
            .tasks
            .get(&label)
            .ok_or_else(|| anyhow!("task not found: {}", canonical_label(&label)))?;
        for dep in &task.deps {
            stack.push(dep.clone());
        }
    }
    Ok(closure)
}
