use tak_core::model::{
    LocalSpec, PolicyDecisionSpec, RemoteRuntimeSpec, RemoteSpec, RemoteTransportKind,
    ResolvedTask, TaskExecutionSpec,
};

use super::run_override_runtime::declared_container_runtime;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RunPlacementSelector {
    Local,
    Remote,
}

pub(super) fn resolved_runtime_for_execution_override(
    task: &ResolvedTask,
    execution: &TaskExecutionSpec,
    placement: RunPlacementSelector,
    container: bool,
    explicit_runtime: Option<&RemoteRuntimeSpec>,
) -> Result<Option<RemoteRuntimeSpec>> {
    match placement {
        RunPlacementSelector::Local if container => {
            resolve_container_runtime_for_execution(
                task,
                execution,
                explicit_runtime,
                format!(
                    "task {} requires --container-image, --container-dockerfile, or TASKS.py defaults.container_runtime when using --container",
                    canonical_label(&task.label)
                ),
            )
            .map(Some)
        }
        RunPlacementSelector::Local => Ok(declared_container_runtime(execution)),
        RunPlacementSelector::Remote => {
            resolve_container_runtime_for_execution(
                task,
                execution,
                explicit_runtime,
                format!(
                    "task {} requires a containerized runtime for --remote; provide --container-image, --container-dockerfile, Execution.Remote(..., runtime=Runtime.Image(...)), or TASKS.py defaults.container_runtime",
                    canonical_label(&task.label)
                ),
            )
            .map(Some)
        }
    }
}

fn resolve_container_runtime_for_execution(
    task: &ResolvedTask,
    execution: &TaskExecutionSpec,
    explicit_runtime: Option<&RemoteRuntimeSpec>,
    missing_runtime_message: String,
) -> Result<RemoteRuntimeSpec> {
    if let Some(runtime) = explicit_runtime {
        return Ok(runtime.clone());
    }
    if let Some(runtime) = declared_container_runtime(execution) {
        return Ok(runtime);
    }
    if let Some(runtime) = task.container_runtime.clone() {
        return Ok(runtime);
    }
    bail!(missing_runtime_message)
}

pub(super) fn rewrite_execution_for_placement(
    execution: &TaskExecutionSpec,
    placement: RunPlacementSelector,
    runtime: Option<RemoteRuntimeSpec>,
) -> TaskExecutionSpec {
    match placement {
        RunPlacementSelector::Local => {
            let mut local = existing_local_spec(execution).unwrap_or_default();
            local.runtime = runtime;
            TaskExecutionSpec::LocalOnly(local)
        }
        RunPlacementSelector::Remote => {
            let mut remote = existing_remote_spec(execution).unwrap_or_else(default_remote_spec);
            remote.runtime = runtime;
            TaskExecutionSpec::RemoteOnly(remote)
        }
    }
}

fn existing_local_spec(execution: &TaskExecutionSpec) -> Option<LocalSpec> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => Some(local.clone()),
        TaskExecutionSpec::ByCustomPolicy {
            decision:
                Some(PolicyDecisionSpec::Local {
                    local: Some(local), ..
                }),
            ..
        } => Some(local.clone()),
        TaskExecutionSpec::RemoteOnly(_)
        | TaskExecutionSpec::ByCustomPolicy { .. }
        | TaskExecutionSpec::UseSession { .. } => None,
    }
}

fn existing_remote_spec(execution: &TaskExecutionSpec) -> Option<RemoteSpec> {
    match execution {
        TaskExecutionSpec::RemoteOnly(remote) => Some(remote.clone()),
        TaskExecutionSpec::ByCustomPolicy {
            decision: Some(PolicyDecisionSpec::Remote { remote, .. }),
            ..
        } => Some(remote.clone()),
        TaskExecutionSpec::LocalOnly(_)
        | TaskExecutionSpec::ByCustomPolicy { .. }
        | TaskExecutionSpec::UseSession { .. } => None,
    }
}

fn default_remote_spec() -> RemoteSpec {
    RemoteSpec {
        pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Any,
        runtime: None,
    }
}

pub(super) fn resolve_run_placement_selector(
    local: bool,
    remote: bool,
) -> Result<Option<RunPlacementSelector>> {
    match (local, remote) {
        (true, true) => bail!("--local and --remote are mutually exclusive"),
        (true, false) => Ok(Some(RunPlacementSelector::Local)),
        (false, true) => Ok(Some(RunPlacementSelector::Remote)),
        (false, false) => Ok(None),
    }
}

pub(super) fn validate_container_flag_usage(
    placement: Option<RunPlacementSelector>,
    container: bool,
    container_image: Option<&str>,
    container_dockerfile: Option<&str>,
    container_build_context: Option<&str>,
) -> Result<()> {
    if container_image.is_some() && container_dockerfile.is_some() {
        bail!("--container-image and --container-dockerfile are mutually exclusive");
    }
    if container_build_context.is_some() && container_dockerfile.is_none() {
        bail!("--container-build-context requires --container-dockerfile");
    }
    if !container
        && placement != Some(RunPlacementSelector::Remote)
        && (container_image.is_some()
            || container_dockerfile.is_some()
            || container_build_context.is_some())
    {
        bail!("container source flags require --remote or --container");
    }
    if container && placement.is_none() {
        bail!("--container requires exactly one of --local or --remote");
    }
    Ok(())
}
