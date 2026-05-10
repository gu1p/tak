use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use tak_core::model::{
    CurrentStateSpec, ResolvedTask, SessionDef, SessionLifetimeSpec, SessionReuseDef,
    SessionReuseSpec, SessionSpec, SessionUseSpec, TaskExecutionDef, TaskExecutionSpec, TaskLabel,
};

use super::{
    MergeState,
    context_resolution::resolve_current_state,
    execution_resolution::{resolve_execution, scoped_session_name},
    output_resolution::resolve_output_selectors,
    session_attached_validation::validate_attached_execution_sessions,
    session_validation::{validate_implicit_session_context, validate_session_execution},
};

pub(crate) fn register_reachable_sessions(
    module_path: &Path,
    package: &str,
    execution: Option<&TaskExecutionDef>,
    state: &mut MergeState,
) -> Result<()> {
    let Some(execution) = execution else {
        return Ok(());
    };
    register_sessions_in_execution(module_path, package, execution, state)
}

fn register_sessions_in_execution(
    module_path: &Path,
    package: &str,
    execution: &TaskExecutionDef,
    state: &mut MergeState,
) -> Result<()> {
    match execution {
        TaskExecutionDef::LocalOnly { local } => {
            if let Some(session) = local.session.as_ref() {
                register_session(module_path, package, session.as_ref().clone(), state)?;
            }
        }
        TaskExecutionDef::RemoteOnly { remote } => {
            if let Some(session) = remote.session.as_ref() {
                register_session(module_path, package, session.as_ref().clone(), state)?;
            }
        }
        TaskExecutionDef::ByExecutionPolicy { placements, .. } => {
            for placement in placements {
                register_sessions_in_execution(module_path, package, placement, state)?;
            }
        }
        TaskExecutionDef::UseSession {
            session: Some(session),
            ..
        } => register_session(module_path, package, session.as_ref().clone(), state)?,
        TaskExecutionDef::ByCustomPolicy { .. } | TaskExecutionDef::UseSession { .. } => {}
    };
    Ok(())
}

fn register_session(
    module_path: &Path,
    package: &str,
    session: SessionDef,
    state: &mut MergeState,
) -> Result<()> {
    let name = scoped_session_name(session.id.trim(), package);
    if state.sessions.contains_key(&name) {
        return Ok(());
    }
    if let Some(execution) = session.execution.as_ref() {
        register_sessions_in_execution(module_path, package, execution, state)?;
    }
    let resolved = resolve_session(session, package)?;
    state
        .session_origins
        .insert(resolved.name.clone(), module_path.to_path_buf());
    state.sessions.insert(resolved.name.clone(), resolved);
    Ok(())
}

pub(crate) fn resolve_session(session: SessionDef, package: &str) -> Result<SessionSpec> {
    let raw_id = session.id.trim().to_string();
    let name = scoped_session_name(&raw_id, package);
    if name.is_empty() {
        bail!("session.id cannot be empty");
    }
    let label = session.name.unwrap_or_else(|| name.clone());
    if session.lifetime.trim() != "per_run" {
        bail!(
            "session `{}` lifetime `{}` is unsupported; expected SessionLifetime.PerRun",
            label,
            session.lifetime
        );
    }

    let execution = session
        .execution
        .map(|execution| resolve_execution(execution, package))
        .transpose()?;
    if let Some(execution) = execution.as_ref() {
        validate_session_execution(&label, execution)?;
    }
    let reuse = resolve_session_reuse(&label, session.reuse, package)?;
    let context = session
        .context
        .map(|context| resolve_current_state(Some(context), package))
        .transpose()?;

    Ok(SessionSpec {
        name,
        display_name: label,
        execution: execution.map(Box::new),
        reuse,
        lifetime: SessionLifetimeSpec::PerRun,
        context,
    })
}

pub(crate) fn resolve_attached_session(
    session: Option<Box<SessionDef>>,
    package: &str,
) -> Result<Option<SessionUseSpec>> {
    session
        .map(|session| resolve_session_use(*session, package))
        .transpose()
}

fn resolve_session_use(session: SessionDef, package: &str) -> Result<SessionUseSpec> {
    let session = resolve_session(session, package)?;
    Ok(session_use_from_spec(&session))
}

pub(crate) fn session_use_from_spec(session: &SessionSpec) -> SessionUseSpec {
    SessionUseSpec {
        name: session.name.clone(),
        display_name: session.display_name.clone(),
        execution: session.execution.clone(),
        reuse: session.reuse.clone(),
        context: session.context.clone(),
    }
}

pub(crate) fn bind_task_sessions(
    tasks: &mut BTreeMap<TaskLabel, ResolvedTask>,
    sessions: &BTreeMap<String, SessionSpec>,
) -> Result<()> {
    let mut contexts = BTreeMap::<String, CurrentStateSpec>::new();
    for (label, task) in tasks {
        if let TaskExecutionSpec::UseSession { name, .. } = &task.execution {
            let session = sessions.get(name.as_str()).ok_or_else(|| {
                anyhow::anyhow!("task(use_session=...) references unknown session `{name}`")
            })?;
            if session.execution.is_none() {
                bail!("task(use_session=...) references session `{name}` without legacy execution");
            }
            if session.context.is_none() {
                validate_implicit_session_context(
                    &mut contexts,
                    name.as_str(),
                    label,
                    &task.context,
                )?;
            }
            task.session = Some(session_use_from_spec(session));
        }
        validate_attached_execution_sessions(&task.execution, &mut contexts, label, &task.context)?;
    }
    Ok(())
}

fn resolve_session_reuse(
    name: &str,
    reuse: SessionReuseDef,
    package: &str,
) -> Result<SessionReuseSpec> {
    match reuse {
        SessionReuseDef::ShareWorkspace => Ok(SessionReuseSpec::ShareWorkspace),
        SessionReuseDef::SharePaths { paths } => {
            if paths.is_empty() {
                bail!("session `{name}` SessionReuse.Paths requires at least one path");
            }
            Ok(SessionReuseSpec::SharePaths {
                paths: resolve_output_selectors(paths, package)?,
            })
        }
        SessionReuseDef::Container => Ok(SessionReuseSpec::Container),
    }
}
