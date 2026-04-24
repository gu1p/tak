use tak_core::model::{TaskExecutionSpec, TaskLabel, WorkspaceSpec};

use super::run_override_runtime::explicit_container_runtime_override;
use super::run_overrides_closure::*;
use super::run_overrides_support::*;
use super::*;

pub(super) fn warn_redundant_remote_container_flag(remote: bool, container: bool) -> bool {
    remote && container
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
    let session_names = sessions_used_by_closure(&overridden, &closure)?;

    for session_name in session_names {
        let task = first_task_using_session(&overridden, &closure, &session_name)?.clone();
        let session = overridden
            .sessions
            .get_mut(&session_name)
            .ok_or_else(|| anyhow!("session not found: {session_name}"))?;
        let selected_placement = placement.expect("placement validated");
        let runtime = resolved_runtime_for_execution_override(
            &task,
            &session.execution,
            selected_placement,
            args.container,
            explicit_runtime.as_ref(),
        )?;
        session.execution =
            rewrite_execution_for_placement(&session.execution, selected_placement, runtime);
    }

    for label in closure {
        let task = overridden
            .tasks
            .get_mut(&label)
            .ok_or_else(|| anyhow!("task not found: {}", canonical_label(&label)))?;
        if let TaskExecutionSpec::UseSession { name, .. } = &task.execution {
            if let Some(session) = overridden.sessions.get(name)
                && let Some(binding) = task.session.as_mut()
            {
                binding.execution = session.execution.clone();
            }
            continue;
        }
        let selected_placement = placement.expect("placement validated");
        let runtime = resolved_runtime_for_execution_override(
            task,
            &task.execution,
            selected_placement,
            args.container,
            explicit_runtime.as_ref(),
        )?;
        task.execution =
            rewrite_execution_for_placement(&task.execution, selected_placement, runtime);
    }

    Ok(overridden)
}
