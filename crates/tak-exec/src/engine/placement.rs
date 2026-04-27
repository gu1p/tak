use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use tak_core::model::{
    ExecutionPlacementSpec, PolicyDecisionSpec, RemoteSelectionSpec, RemoteSpec, ResolvedTask,
    TaskExecutionSpec, TaskLabel,
};
use tak_loader::evaluate_named_policy_decision;

use super::{NoMatchingRemoteError, PlacementMode, remote_models::TaskPlacement};
use crate::client_remotes::configured_remote_targets;

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
            resolve_session_candidates(task, &session.name, &session.execution)
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

fn resolve_session_candidates(
    task: &ResolvedTask,
    session_name: &str,
    execution: &TaskExecutionSpec,
) -> Result<Vec<PlacementCandidate>> {
    match execution {
        TaskExecutionSpec::LocalOnly(local) => Ok(vec![PlacementCandidate::Ready(Box::new(
            local_task_placement(local.clone(), None),
        ))]),
        TaskExecutionSpec::RemoteOnly(remote) => {
            Ok(vec![remote_task_candidate(task, remote, None)?])
        }
        TaskExecutionSpec::ByExecutionPolicy { name, placements } => placements
            .iter()
            .map(|placement| execution_policy_candidate(task, name, placement))
            .collect(),
        TaskExecutionSpec::ByCustomPolicy { .. } => {
            bail!("session `{session_name}` uses unsupported ByCustomPolicy execution")
        }
        TaskExecutionSpec::UseSession { .. } => {
            bail!("session `{session_name}` cannot use another session")
        }
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

fn local_task_placement(
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
        local: Some(local),
    }
}

fn tasks_file_for_label(workspace_root: &Path, label: &TaskLabel) -> PathBuf {
    if label.package == "//" {
        return workspace_root.join("TASKS.py");
    }

    let package = label.package.trim_start_matches("//");
    workspace_root.join(package).join("TASKS.py")
}

fn remote_task_candidate(
    task: &ResolvedTask,
    remote: &RemoteSpec,
    reason: Option<String>,
) -> Result<PlacementCandidate> {
    let remote = materialize_effective_remote_spec(task, remote)?;
    let selection = configured_remote_targets(&remote)?;
    if selection.matched_targets.is_empty() {
        return Ok(PlacementCandidate::Unavailable(
            NoMatchingRemoteError::new(
                canonical_task_label(&task.label),
                &remote,
                selection.configured_remote_count,
                selection.enabled_remote_count,
                selection.enabled_remotes,
            )
            .into(),
        ));
    }
    Ok(PlacementCandidate::Ready(Box::new(TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: selection.matched_targets,
        remote_selection: remote.selection,
        decision_reason: reason,
        local: None,
    })))
}

fn materialize_effective_remote_spec(
    task: &ResolvedTask,
    remote: &RemoteSpec,
) -> Result<RemoteSpec> {
    if remote.runtime.is_some() {
        return Ok(remote.clone());
    }
    if let Some(runtime) = task.container_runtime.clone() {
        let mut remote = remote.clone();
        remote.runtime = Some(runtime);
        return Ok(remote);
    }

    bail!(
        "task {} requires a containerized runtime for remote execution; provide Execution.Remote(..., runtime=Runtime.Image(...)), Decision.remote(..., runtime=Runtime.Image(...)), or TASKS.py defaults.container_runtime",
        canonical_task_label(&task.label)
    )
}

fn canonical_task_label(label: &TaskLabel) -> String {
    format!("{}:{}", label.package, label.name)
}
