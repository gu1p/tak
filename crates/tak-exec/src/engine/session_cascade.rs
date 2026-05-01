use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use tak_core::model::{ResolvedTask, SessionUseSpec, TaskExecutionSpec, TaskLabel, WorkspaceSpec};

use super::TaskOutputObserver;
use super::remote_models::TaskPlacement;
use super::session_cascade_selection::select_cascade_execution;

#[derive(Clone)]
pub(crate) struct ExecutionCascadeOverride {
    pub(crate) execution: TaskExecutionSpec,
    pub(crate) placement: Option<TaskPlacement>,
    pub(crate) fingerprint: String,
    pub(crate) root: TaskLabel,
}

pub(crate) async fn resolve_cascaded_executions(
    spec: &WorkspaceSpec,
    required: &BTreeSet<TaskLabel>,
    workspace_root: &Path,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<BTreeMap<TaskLabel, ExecutionCascadeOverride>> {
    let mut resolved = BTreeMap::new();
    for label in required {
        let task = task_for_label(spec, label)?;
        if !task_requests_execution_cascade(task) {
            continue;
        }
        let cascade = select_cascade_execution(task, workspace_root, output_observer).await?;
        let mut closure = BTreeSet::new();
        collect_dependency_closure(spec, label, &mut closure)?;
        for member_label in closure {
            if required.contains(&member_label) {
                bind_cascaded_execution(&member_label, &cascade, &mut resolved)?;
            }
        }
    }
    Ok(resolved)
}

pub(crate) fn task_with_execution_override(
    task: &ResolvedTask,
    cascade: Option<&ExecutionCascadeOverride>,
) -> Option<ResolvedTask> {
    let cascade = cascade?;
    let mut effective = task.clone();
    effective.execution = cascade.execution.clone();
    effective.session = None;
    effective.cascade_execution = false;
    Some(effective)
}

pub(crate) fn task_with_session_context(
    task: &ResolvedTask,
    selected_session: Option<&SessionUseSpec>,
) -> Option<ResolvedTask> {
    let context = selected_session
        .or(task.session.as_ref())?
        .context
        .as_ref()?;
    if &task.context == context {
        return None;
    }
    let mut effective = task.clone();
    effective.context = context.clone();
    Some(effective)
}

fn task_requests_execution_cascade(task: &ResolvedTask) -> bool {
    task.cascade_execution
        || matches!(
            task.execution,
            TaskExecutionSpec::UseSession { cascade: true, .. }
        )
}

fn bind_cascaded_execution(
    label: &TaskLabel,
    cascade: &ExecutionCascadeOverride,
    resolved: &mut BTreeMap<TaskLabel, ExecutionCascadeOverride>,
) -> Result<()> {
    if let Some(previous) = resolved.get(label) {
        if previous.fingerprint == cascade.fingerprint {
            return Ok(());
        }
        bail!(
            "execution cascade conflict: task {} is reached by {} and {} with different executions",
            canonical_label(label),
            canonical_label(&previous.root),
            canonical_label(&cascade.root)
        );
    }
    resolved.insert(label.clone(), cascade.clone());
    Ok(())
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

fn task_for_label<'a>(spec: &'a WorkspaceSpec, label: &TaskLabel) -> Result<&'a ResolvedTask> {
    spec.tasks
        .get(label)
        .ok_or_else(|| anyhow!("missing task definition for label {label}"))
}

fn canonical_label(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
