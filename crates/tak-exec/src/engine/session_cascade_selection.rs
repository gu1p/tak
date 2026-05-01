use std::path::Path;

use anyhow::{Result, anyhow};
use tak_core::model::{ResolvedTask, SessionUseSpec, TaskExecutionSpec};
use uuid::Uuid;

use super::attempt_placement::preflight_task_placement;
use super::remote_models::TaskPlacement;
use super::session_cascade::ExecutionCascadeOverride;
use super::session_cascade_context::{
    apply_root_context_to_session, execution_with_root_context, session_for_cascade_root,
};
use super::{PlacementMode, TaskOutputObserver};

pub(crate) async fn select_cascade_execution(
    task: &ResolvedTask,
    workspace_root: &Path,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<ExecutionCascadeOverride> {
    if !should_preflight_cascade_execution(task)? {
        let execution = fixed_cascade_execution(task)?;
        return Ok(ExecutionCascadeOverride {
            fingerprint: execution_fingerprint(&execution),
            execution,
            placement: None,
            root: task.label.clone(),
        });
    }
    let task_run_id = Uuid::new_v4().to_string();
    let mut placement =
        preflight_task_placement(task, workspace_root, &task_run_id, 1, output_observer).await?;
    pin_selected_remote_target(&mut placement);
    apply_root_context_to_session(task, &mut placement);
    let execution = execution_from_placement(&placement)?;
    Ok(ExecutionCascadeOverride {
        fingerprint: execution_fingerprint(&execution),
        execution,
        placement: Some(placement),
        root: task.label.clone(),
    })
}

fn should_preflight_cascade_execution(task: &ResolvedTask) -> Result<bool> {
    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { .. } | TaskExecutionSpec::ByCustomPolicy { .. } => {
            Ok(true)
        }
        TaskExecutionSpec::UseSession { name, .. } => {
            let session = task.session.as_ref().ok_or_else(|| {
                anyhow!(
                    "task {} references session `{name}` but no resolved session is attached",
                    task.label
                )
            })?;
            let execution = session.execution.as_deref().ok_or_else(|| {
                anyhow!(
                    "task {} references session `{name}` but the session has no legacy execution",
                    task.label
                )
            })?;
            Ok(matches!(
                execution,
                TaskExecutionSpec::ByExecutionPolicy { .. }
                    | TaskExecutionSpec::ByCustomPolicy { .. }
            ))
        }
        TaskExecutionSpec::LocalOnly(_) | TaskExecutionSpec::RemoteOnly(_) => Ok(false),
    }
}

fn fixed_cascade_execution(task: &ResolvedTask) -> Result<TaskExecutionSpec> {
    let execution = match &task.execution {
        TaskExecutionSpec::UseSession { name, .. } => {
            let session = task.session.as_ref().ok_or_else(|| {
                anyhow!(
                    "task {} references session `{name}` but no resolved session is attached",
                    task.label
                )
            })?;
            let execution = session.execution.as_deref().ok_or_else(|| {
                anyhow!(
                    "task {} references session `{name}` but the session has no legacy execution",
                    task.label
                )
            })?;
            execution_with_session(
                (*execution).clone(),
                session_for_cascade_root(task, session),
            )
        }
        other => other.clone(),
    };
    Ok(execution_with_root_context(execution, task))
}

fn execution_with_session(
    execution: TaskExecutionSpec,
    session: SessionUseSpec,
) -> TaskExecutionSpec {
    match execution {
        TaskExecutionSpec::LocalOnly(mut local) => {
            local.session = Some(session);
            TaskExecutionSpec::LocalOnly(local)
        }
        TaskExecutionSpec::RemoteOnly(mut remote) => {
            remote.session = Some(session);
            TaskExecutionSpec::RemoteOnly(remote)
        }
        other => other,
    }
}

fn pin_selected_remote_target(placement: &mut TaskPlacement) {
    let Some(selected) = placement.strict_remote_target.clone() else {
        return;
    };
    placement.ordered_remote_targets = vec![selected];
}

fn execution_from_placement(placement: &TaskPlacement) -> Result<TaskExecutionSpec> {
    match placement.placement_mode {
        PlacementMode::Local => {
            let local = placement
                .local
                .clone()
                .ok_or_else(|| anyhow!("missing local placement for cascade"))?;
            Ok(TaskExecutionSpec::LocalOnly(local))
        }
        PlacementMode::Remote => {
            let remote = placement
                .remote
                .clone()
                .ok_or_else(|| anyhow!("missing remote placement for cascade"))?;
            Ok(TaskExecutionSpec::RemoteOnly(remote))
        }
    }
}

fn execution_fingerprint(execution: &TaskExecutionSpec) -> String {
    format!("{execution:?}")
}
