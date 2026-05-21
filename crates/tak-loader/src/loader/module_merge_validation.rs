use anyhow::{Result, bail};
use tak_core::model::{
    ExecutionPlacementSpec, PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSpec, TaskExecutionSpec,
};

use super::remote_validation::validate_remote_runtime_limits;

pub(crate) fn validate_remote_session_runtime(
    execution: &TaskExecutionSpec,
    default_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<()> {
    match execution {
        TaskExecutionSpec::RemoteOnly(remote) => validate_session_remote(remote, default_runtime),
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => {
            for placement in placements {
                if let ExecutionPlacementSpec::Remote(remote) = placement {
                    validate_session_remote(remote, default_runtime)?;
                }
            }
            Ok(())
        }
        TaskExecutionSpec::LocalOnly(_)
        | TaskExecutionSpec::ByCustomPolicy { .. }
        | TaskExecutionSpec::UseSession { .. } => Ok(()),
    }
}

fn validate_session_remote(
    remote: &RemoteSpec,
    default_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<()> {
    if remote.session.is_none() || remote.runtime.is_some() || default_runtime.is_some() {
        return Ok(());
    }
    bail!("Execution.Remote(session=...) requires a container or Defaults(container=...)")
}

pub(crate) fn validate_remote_resource_limits(
    execution: &TaskExecutionSpec,
    default_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<()> {
    match execution {
        TaskExecutionSpec::RemoteOnly(remote) => validate_effective_limits(remote, default_runtime),
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => {
            for placement in placements {
                if let ExecutionPlacementSpec::Remote(remote) = placement {
                    validate_effective_limits(remote, default_runtime)?;
                }
            }
            Ok(())
        }
        TaskExecutionSpec::ByCustomPolicy {
            decision: Some(PolicyDecisionSpec::Remote { remote, .. }),
            ..
        } => validate_effective_limits(remote, default_runtime),
        TaskExecutionSpec::LocalOnly(_)
        | TaskExecutionSpec::ByCustomPolicy { .. }
        | TaskExecutionSpec::UseSession { .. } => Ok(()),
    }
}

fn validate_effective_limits(
    remote: &RemoteSpec,
    default_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<()> {
    let Some(runtime) = remote.runtime.as_ref().or(default_runtime) else {
        return Ok(());
    };
    validate_remote_runtime_limits(runtime, "remote execution")
}
