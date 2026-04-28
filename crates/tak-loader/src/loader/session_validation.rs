use std::collections::BTreeMap;

use anyhow::{Result, bail};
use tak_core::model::{
    CurrentStateSpec, ExecutionPlacementSpec, RemoteRuntimeSpec, TaskExecutionSpec, TaskLabel,
};

pub(crate) fn validate_session_execution(name: &str, execution: &TaskExecutionSpec) -> Result<()> {
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

pub(crate) fn validate_implicit_session_context(
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
