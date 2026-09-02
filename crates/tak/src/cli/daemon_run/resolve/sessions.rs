use std::collections::BTreeMap;

use anyhow::{Result, bail};
use tak_core::v2::{Affinity, AuthoredModule, AuthoredTask, Execution, Session};

use super::graph::canonical;

pub(super) struct Binding {
    pub(super) execution: Option<Execution>,
    pub(super) session: Option<Session>,
    pub(super) affinity: Option<Affinity>,
}

pub(super) fn bindings(
    module: &AuthoredModule,
    tasks: &[&AuthoredTask],
) -> Result<BTreeMap<String, Binding>> {
    let known = tasks
        .iter()
        .map(|task| Ok((canonical(&task.name)?, *task)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut cascaded = BTreeMap::<String, Session>::new();
    for task in tasks.iter().filter(|task| task.cascade_session) {
        let session = task
            .session
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cascading task `{}` has no session", task.name))?;
        assign(&canonical(&task.name)?, session, &known, &mut cascaded)?;
    }
    tasks
        .iter()
        .map(|task| {
            let id = canonical(&task.name)?;
            Ok((id.clone(), bind(module, task, cascaded.get(&id))?))
        })
        .collect()
}

fn assign(
    task_id: &str,
    session: &Session,
    known: &BTreeMap<String, &AuthoredTask>,
    cascaded: &mut BTreeMap<String, Session>,
) -> Result<()> {
    if let Some(existing) = cascaded.get(task_id) {
        if existing != session {
            bail!("task `{task_id}` belongs to conflicting cascading sessions")
        }
        return Ok(());
    }
    cascaded.insert(task_id.into(), session.clone());
    for dependency in &known[task_id].deps {
        assign(&canonical(dependency)?, session, known, cascaded)?;
    }
    Ok(())
}

fn bind(
    module: &AuthoredModule,
    task: &AuthoredTask,
    cascaded: Option<&Session>,
) -> Result<Binding> {
    if let (Some(direct), Some(inherited)) = (task.session.as_ref(), cascaded)
        && direct != inherited
    {
        bail!("task `{}` overrides its cascading session", task.name)
    }
    let execution = if let Some(session) = cascaded {
        let inherited = session.execution.as_deref().ok_or_else(|| {
            anyhow::anyhow!("cascading session `{}` has no execution", session.id)
        })?;
        if task
            .execution
            .as_ref()
            .is_some_and(|explicit| explicit != inherited)
        {
            bail!("task `{}` overrides its cascading execution", task.name)
        }
        Some(inherited.clone())
    } else {
        effective_execution(module, task)
    };
    let session = cascaded.cloned().or_else(|| {
        task.session
            .clone()
            .or_else(|| execution.as_ref().and_then(attached_session).cloned())
    });
    let affinity = match &session {
        Some(session) => session.effective_affinity(task.affinity.as_ref())?,
        None => task.affinity.clone(),
    };
    Ok(Binding {
        execution,
        session,
        affinity,
    })
}

fn effective_execution(module: &AuthoredModule, task: &AuthoredTask) -> Option<Execution> {
    task.execution
        .clone()
        .or_else(|| {
            task.session
                .as_ref()
                .and_then(|session| session.execution.as_deref().cloned())
        })
        .or_else(|| module.defaults.execution.clone())
}

fn attached_session(execution: &Execution) -> Option<&Session> {
    match execution {
        Execution::LocalOnly { local } => local.session.as_deref(),
        Execution::RemoteOnly { remote } => remote.session.as_deref(),
        Execution::FirstAvailable { placements, .. } => {
            placements.iter().find_map(attached_session)
        }
    }
}
