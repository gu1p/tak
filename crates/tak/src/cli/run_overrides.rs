use std::collections::BTreeSet;
use tak_core::model::{
    LocalSpec, RemoteSpec, RemoteTransportKind, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

use super::run_override_runtime::{
    explicit_container_runtime_override, resolve_container_runtime_for_task,
};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunPlacementSelector {
    Local,
    Remote,
}

pub(super) struct RunExecutionOverrideArgs<'a> {
    pub local: bool,
    pub remote: bool,
    pub container: bool,
    pub container_image: Option<&'a str>,
    pub container_dockerfile: Option<&'a str>,
    pub container_build_context: Option<&'a str>,
}

pub(super) fn apply_run_execution_overrides(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
    args: RunExecutionOverrideArgs<'_>,
) -> Result<WorkspaceSpec> {
    let placement = resolve_run_placement_selector(args.local, args.remote)?;
    validate_container_flag_usage(
        placement,
        args.container,
        args.container_image,
        args.container_dockerfile,
        args.container_build_context,
    )?;
    if placement.is_none() {
        return Ok(spec.clone());
    }

    let explicit_runtime = explicit_container_runtime_override(
        args.container_image,
        args.container_dockerfile,
        args.container_build_context,
    )?;
    let closure = target_closure(spec, targets)?;
    let mut overridden = spec.clone();

    for label in closure {
        let task = overridden
            .tasks
            .get_mut(&label)
            .ok_or_else(|| anyhow!("task not found: {}", canonical_label(&label)))?;
        let runtime = args
            .container
            .then(|| resolve_container_runtime_for_task(task, explicit_runtime.as_ref()))
            .transpose()?;
        task.execution = match placement.expect("placement validated") {
            RunPlacementSelector::Local => TaskExecutionSpec::LocalOnly(LocalSpec {
                runtime,
                ..LocalSpec::default()
            }),
            RunPlacementSelector::Remote => TaskExecutionSpec::RemoteOnly(RemoteSpec {
                pool: None,
                required_tags: Vec::new(),
                required_capabilities: Vec::new(),
                transport_kind: RemoteTransportKind::Any,
                runtime,
            }),
        };
    }

    Ok(overridden)
}

fn resolve_run_placement_selector(
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

fn validate_container_flag_usage(
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
        && (container_image.is_some()
            || container_dockerfile.is_some()
            || container_build_context.is_some())
    {
        bail!("container source flags require --container");
    }
    if container && placement.is_none() {
        bail!("--container requires exactly one of --local or --remote");
    }
    Ok(())
}

fn target_closure(spec: &WorkspaceSpec, targets: &[TaskLabel]) -> Result<BTreeSet<TaskLabel>> {
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
