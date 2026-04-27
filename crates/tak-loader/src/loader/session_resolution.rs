use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use tak_core::model::{
    CurrentStateSpec, ExecutionPlacementSpec, ExecutionPolicySpec, RemoteRuntimeSpec, ResolvedTask,
    SessionDef, SessionLifetimeSpec, SessionReuseDef, SessionReuseSpec, SessionSpec,
    SessionUseSpec, TaskExecutionSpec, TaskLabel,
};

use super::{
    MergeState, context_resolution::resolve_current_state,
    execution_policy_resolution::resolve_execution_policy_reference,
    execution_resolution::resolve_execution, output_resolution::resolve_output_selectors,
};

pub(crate) fn register_module_sessions(
    module_path: &Path,
    package: &str,
    sessions: Vec<SessionDef>,
    state: &mut MergeState,
) -> Result<()> {
    for session in sessions {
        let resolved = resolve_session(session, package, &state.execution_policies)?;
        if let Some(previous) = state.session_origins.get(&resolved.name) {
            bail!(
                "duplicate session definition: {}\nfirst defined in {}\nconflicts with {}",
                resolved.name,
                previous.display(),
                module_path.display()
            );
        }
        state
            .session_origins
            .insert(resolved.name.clone(), module_path.to_path_buf());
        state.sessions.insert(resolved.name.clone(), resolved);
    }
    Ok(())
}

pub(crate) fn resolve_session(
    session: SessionDef,
    package: &str,
    policies: &BTreeMap<String, ExecutionPolicySpec>,
) -> Result<SessionSpec> {
    let name = session.name.trim().to_string();
    if name.is_empty() {
        bail!("session.name cannot be empty");
    }
    if session.lifetime.trim() != "per_run" {
        bail!(
            "session `{}` lifetime `{}` is unsupported; expected SessionLifetime.PerRun",
            name,
            session.lifetime
        );
    }

    if session.execution.is_some() && session.execution_policy.is_some() {
        bail!("session `{name}` cannot set both execution and execution_policy");
    }
    let execution = match (session.execution, session.execution_policy) {
        (Some(execution), None) => resolve_execution(execution, package)?,
        (None, Some(policy_name)) => resolve_execution_policy_reference(&policy_name, policies)?,
        (None, None) => bail!("session `{name}` requires execution or execution_policy"),
        (Some(_), Some(_)) => unreachable!("mixed session execution rejected above"),
    };
    validate_session_execution(&name, &execution)?;
    let reuse = resolve_session_reuse(&name, session.reuse, package)?;
    let context = session
        .context
        .map(|context| resolve_current_state(Some(context), package))
        .transpose()?;

    Ok(SessionSpec {
        name,
        execution,
        reuse,
        lifetime: SessionLifetimeSpec::PerRun,
        context,
    })
}

pub(crate) fn bind_task_sessions(
    tasks: &mut BTreeMap<TaskLabel, ResolvedTask>,
    sessions: &BTreeMap<String, SessionSpec>,
) -> Result<()> {
    let mut contexts = BTreeMap::<String, CurrentStateSpec>::new();
    for (label, task) in tasks {
        let TaskExecutionSpec::UseSession { name, .. } = &task.execution else {
            continue;
        };
        let session = sessions.get(name).ok_or_else(|| {
            anyhow::anyhow!("Execution.Session references unknown session `{name}`")
        })?;
        if session.context.is_none() {
            validate_implicit_session_context(&mut contexts, name, label, &task.context)?;
        }
        task.session = Some(SessionUseSpec {
            name: name.clone(),
            execution: session.execution.clone(),
            reuse: session.reuse.clone(),
            context: session.context.clone(),
        });
    }
    Ok(())
}

fn validate_session_execution(name: &str, execution: &TaskExecutionSpec) -> Result<()> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => {
            validate_session_runtime(name, local.runtime.as_ref())
        }
        TaskExecutionSpec::RemoteOnly(remote) => {
            validate_session_runtime(name, remote.runtime.as_ref())
        }
        TaskExecutionSpec::ByCustomPolicy { .. } => {
            bail!("session `{name}` execution cannot use ByCustomPolicy in v1")
        }
        TaskExecutionSpec::ByExecutionPolicy {
            name: policy_name,
            placements,
        } => validate_session_policy_execution(name, policy_name, placements),
        TaskExecutionSpec::UseSession { .. } => {
            bail!("session `{name}` execution cannot use UseSession")
        }
    }
}

fn validate_session_policy_execution(
    name: &str,
    policy_name: &str,
    placements: &[ExecutionPlacementSpec],
) -> Result<()> {
    for placement in placements {
        let runtime = match placement {
            ExecutionPlacementSpec::Local(local) => local.runtime.as_ref(),
            ExecutionPlacementSpec::Remote(remote) => remote.runtime.as_ref(),
        };
        if runtime.is_none() {
            bail!(
                "session `{name}` execution_policy `{policy_name}` requires every placement to use a containerized runtime"
            );
        }
    }
    Ok(())
}

fn validate_session_runtime(name: &str, runtime: Option<&RemoteRuntimeSpec>) -> Result<()> {
    if runtime.is_some() {
        return Ok(());
    }
    bail!("session `{name}` execution requires a containerized runtime")
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
    }
}

fn validate_implicit_session_context(
    contexts: &mut BTreeMap<String, CurrentStateSpec>,
    session_name: &str,
    label: &TaskLabel,
    context: &CurrentStateSpec,
) -> Result<()> {
    let Some(previous) = contexts.get(session_name) else {
        contexts.insert(session_name.to_string(), context.clone());
        return Ok(());
    };
    if previous == context {
        return Ok(());
    }
    bail!(
        "session `{session_name}` has no context; task {label} does not match the first CurrentState used by the session"
    )
}
