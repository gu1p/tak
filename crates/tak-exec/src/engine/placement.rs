use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use tak_core::model::{
    ExecutionPlacementSpec, PolicyDecisionSpec, RemoteSelectionSpec, ResolvedTask,
    TaskExecutionSpec, TaskLabel,
};
use tak_loader::evaluate_named_policy_decision;

use super::placement_remote::remote_task_candidate;
use super::placement_session::resolve_session_candidates;
use super::{PlacementMode, remote_models::TaskPlacement};

pub(crate) enum PlacementCandidate {
    Ready(Box<TaskPlacement>),
    Unavailable(anyhow::Error),
}

pub(crate) fn resolve_task_placement_candidates(
    task: &ResolvedTask,
    workspace_root: &Path,
) -> Result<Vec<PlacementCandidate>> {
    match &task.execution {
        TaskExecutionSpec::LocalOnly(local) => Ok(vec![PlacementCandidate::Ready(Box::new(
            local_task_placement(local.clone(), None),
        ))]),
        TaskExecutionSpec::RemoteOnly(remote) => {
            Ok(vec![remote_task_candidate(task, remote, None)?])
        }
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision,
        } => resolve_custom_policy_candidate(task, workspace_root, policy_name, decision),
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => placements
            .iter()
            .map(|placement| execution_policy_candidate(task, name, placement))
            .collect(),
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
            resolve_session_candidates(task, session, execution)
        }
    }
}

fn resolve_custom_policy_candidate(
    task: &ResolvedTask,
    workspace_root: &Path,
    policy_name: &str,
    decision: &Option<PolicyDecisionSpec>,
) -> Result<Vec<PlacementCandidate>> {
    let resolved_decision = if let Some(decision) = decision.as_ref() {
        decision.clone()
    } else {
        let tasks_file = tasks_file_for_label(workspace_root, &task.label);
        evaluate_named_policy_decision(&tasks_file, &task.label.package, policy_name).with_context(
            || {
                format!(
                    "runtime policy evaluation failed for task {} (policy={policy_name})",
                    task.label
                )
            },
        )?
    };
    match &resolved_decision {
        PolicyDecisionSpec::Local { reason, local } => {
            Ok(vec![PlacementCandidate::Ready(Box::new(
                local_task_placement(local.clone().unwrap_or_default(), Some(reason.clone())),
            ))])
        }
        PolicyDecisionSpec::Remote { reason, remote } => Ok(vec![remote_task_candidate(
            task,
            remote,
            Some(reason.clone()),
        )?]),
    }
}

fn execution_policy_candidate(
    task: &ResolvedTask,
    policy_name: &str,
    placement: &ExecutionPlacementSpec,
) -> Result<PlacementCandidate> {
    match placement {
        ExecutionPlacementSpec::Local(local) => Ok(PlacementCandidate::Ready(Box::new(
            local_task_placement(local.clone(), Some(policy_name.to_string())),
        ))),
        ExecutionPlacementSpec::Remote(remote) => {
            remote_task_candidate(task, remote, Some(policy_name.to_string()))
        }
    }
}

pub(super) fn local_task_placement(
    local: tak_core::model::LocalSpec,
    reason: Option<String>,
) -> TaskPlacement {
    let _ = local.max_parallel_tasks;
    let _ = &local.id;
    TaskPlacement {
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: Vec::new(),
        remote_selection: RemoteSelectionSpec::Sequential,
        decision_reason: reason,
        session: local.session.clone(),
        local: Some(local),
        remote: None,
    }
}

fn tasks_file_for_label(workspace_root: &Path, label: &TaskLabel) -> PathBuf {
    if label.package == "//" {
        return workspace_root.join("TASKS.py");
    }

    let package = label.package.trim_start_matches("//");
    workspace_root.join(package).join("TASKS.py")
}
