use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::{NeedDef, OutputSelectorSpec, ResolvedTask, TaskLabel, WorkspaceSpec};

use crate::engine::session_cascade::ExecutionCascadeOverride;

pub(super) fn dependency_closure(
    spec: &WorkspaceSpec,
    root: &TaskLabel,
) -> Result<BTreeSet<TaskLabel>> {
    let mut closure = BTreeSet::new();
    collect_dependency_closure(spec, root, &mut closure)?;
    Ok(closure)
}

fn collect_dependency_closure(
    spec: &WorkspaceSpec,
    label: &TaskLabel,
    closure: &mut BTreeSet<TaskLabel>,
) -> Result<()> {
    if !closure.insert(label.clone()) {
        return Ok(());
    }
    let task = task_for_label(spec, label)?;
    for dep in &task.deps {
        collect_dependency_closure(spec, dep, closure)?;
    }
    Ok(())
}

pub(super) fn fused_task(
    spec: &WorkspaceSpec,
    root: &TaskLabel,
    members: &[TaskLabel],
    cascade: &ExecutionCascadeOverride,
) -> Result<ResolvedTask> {
    let root_task = task_for_label(spec, root)?;
    let mut task = root_task.clone();
    task.deps.clear();
    task.steps.clear();
    task.needs = merged_needs(spec, members)?;
    task.outputs = merged_outputs(spec, members)?;
    task.execution = cascade.execution.clone();
    task.session = None;
    task.cascade_execution = false;
    Ok(task)
}

pub(super) fn fused_members(
    spec: &WorkspaceSpec,
    members: &[TaskLabel],
) -> Result<Vec<ResolvedTask>> {
    members
        .iter()
        .map(|label| task_for_label(spec, label).cloned())
        .collect()
}

fn merged_needs(spec: &WorkspaceSpec, members: &[TaskLabel]) -> Result<Vec<NeedDef>> {
    let mut merged = Vec::<NeedDef>::new();
    for label in members {
        for need in &task_for_label(spec, label)?.needs {
            if let Some(existing) = merged
                .iter_mut()
                .find(|existing| existing.limiter == need.limiter)
            {
                existing.slots = existing.slots.max(need.slots);
            } else {
                merged.push(need.clone());
            }
        }
    }
    Ok(merged)
}

fn merged_outputs(spec: &WorkspaceSpec, members: &[TaskLabel]) -> Result<Vec<OutputSelectorSpec>> {
    let mut outputs = Vec::<OutputSelectorSpec>::new();
    for label in members {
        for output in &task_for_label(spec, label)?.outputs {
            if !outputs.contains(output) {
                outputs.push(output.clone());
            }
        }
    }
    Ok(outputs)
}

fn task_for_label<'a>(spec: &'a WorkspaceSpec, label: &TaskLabel) -> Result<&'a ResolvedTask> {
    spec.tasks
        .get(label)
        .ok_or_else(|| anyhow::anyhow!("missing task definition for label {label}"))
}

pub(super) fn canonical_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
