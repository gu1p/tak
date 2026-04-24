use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, anyhow, bail};
use tak_core::model::{ResolvedTask, SessionUseSpec, TaskExecutionSpec, TaskLabel, WorkspaceSpec};

pub(crate) fn resolve_cascaded_sessions(
    spec: &WorkspaceSpec,
    required: &BTreeSet<TaskLabel>,
) -> Result<BTreeMap<TaskLabel, SessionUseSpec>> {
    let mut resolved = BTreeMap::new();
    for label in required {
        let task = task_for_label(spec, label)?;
        let TaskExecutionSpec::UseSession { cascade: true, .. } = &task.execution else {
            continue;
        };
        let session = task.session.as_ref().ok_or_else(|| {
            anyhow!(
                "task {} declares cascading session but no resolved session is attached",
                task.label
            )
        })?;
        let session = session_for_cascade_root(task, session);
        let mut closure = BTreeSet::new();
        collect_dependency_closure(spec, label, &mut closure)?;
        for member_label in closure {
            if required.contains(&member_label) {
                bind_cascaded_session(spec, &member_label, &session, &mut resolved)?;
            }
        }
    }
    Ok(resolved)
}

pub(crate) fn task_with_session_override(
    task: &ResolvedTask,
    session: Option<&SessionUseSpec>,
) -> Option<ResolvedTask> {
    let session = session?;
    let mut effective = task.clone();
    effective.execution = TaskExecutionSpec::UseSession {
        name: session.name.clone(),
        cascade: false,
    };
    effective.session = Some(session.clone());
    Some(effective)
}

pub(crate) fn task_with_session_context(task: &ResolvedTask) -> Option<ResolvedTask> {
    let context = task.session.as_ref()?.context.as_ref()?;
    if &task.context == context {
        return None;
    }
    let mut effective = task.clone();
    effective.context = context.clone();
    Some(effective)
}

fn session_for_cascade_root(task: &ResolvedTask, session: &SessionUseSpec) -> SessionUseSpec {
    let mut session = session.clone();
    if session.context.is_none() {
        session.context = Some(task.context.clone());
    }
    session
}

fn bind_cascaded_session(
    spec: &WorkspaceSpec,
    label: &TaskLabel,
    session: &SessionUseSpec,
    resolved: &mut BTreeMap<TaskLabel, SessionUseSpec>,
) -> Result<()> {
    let task = task_for_label(spec, label)?;
    if let Some(explicit) = task.session.as_ref()
        && explicit.name != session.name
    {
        bail!(
            "session cascade conflict: task {} is reached by cascading session `{}` but declares session `{}`",
            canonical_label(label),
            session.name,
            explicit.name
        );
    }
    if let Some(previous) = resolved.get(label) {
        if previous.name == session.name && previous.context == session.context {
            return Ok(());
        }
        bail!(
            "session cascade conflict: task {} inherits session `{}` and session `{}`",
            canonical_label(label),
            previous.name,
            session.name
        );
    }
    resolved.insert(label.clone(), session.clone());
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
