/// Resolves the execution constructor into current placement metadata and validates support.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_task_placement(task: &ResolvedTask, workspace_root: &Path) -> Result<TaskPlacement> {
    match &task.execution {
        TaskExecutionSpec::LocalOnly(local) => {
            // Local constructor metadata is validated by the loader and preserved for summaries.
            let _ = local.max_parallel_tasks;
            let _ = &local.id;
            Ok(TaskPlacement {
                placement_mode: PlacementMode::Local,
                remote_node_id: None,
                strict_remote_target: None,
                ordered_remote_targets: Vec::new(),
                decision_reason: None,
                local: Some(local.clone()),
            })
        }
        TaskExecutionSpec::RemoteOnly(remote) => remote_task_placement(task, remote, None),
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let resolved_decision = if let Some(decision) = decision.as_ref() {
                decision.clone()
            } else {
                let tasks_file = tasks_file_for_label(workspace_root, &task.label);
                evaluate_named_policy_decision(&tasks_file, &task.label.package, policy_name)
                    .with_context(|| {
                    format!(
                        "runtime policy evaluation failed for task {} (policy={policy_name})",
                        task.label
                    )
                })?
            };
            match &resolved_decision {
                PolicyDecisionSpec::Local { reason, local } => Ok(TaskPlacement {
                    placement_mode: PlacementMode::Local,
                    remote_node_id: None,
                    strict_remote_target: None,
                    ordered_remote_targets: Vec::new(),
                    decision_reason: Some(reason.clone()),
                    local: Some(local.clone().unwrap_or_default()),
                }),
                PolicyDecisionSpec::Remote { reason, remote } => {
                    remote_task_placement(task, remote, Some(reason.clone()))
                }
            }
        }
    }
}

fn tasks_file_for_label(workspace_root: &Path, label: &TaskLabel) -> PathBuf {
    if label.package == "//" {
        return workspace_root.join("TASKS.py");
    }

    let package = label.package.trim_start_matches("//");
    workspace_root.join(package).join("TASKS.py")
}

fn remote_task_placement(
    task: &ResolvedTask,
    remote: &RemoteSpec,
    reason: Option<String>,
) -> Result<TaskPlacement> {
    let selection = configured_remote_targets(remote)?;
    if selection.matched_targets.is_empty() {
        return Err(NoMatchingRemoteError::new(
            canonical_task_label(&task.label),
            remote,
            selection.configured_remote_count,
            selection.enabled_remote_count,
            selection.enabled_remotes,
        )
        .into());
    }
    Ok(TaskPlacement {
        placement_mode: PlacementMode::Remote,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: selection.matched_targets,
        decision_reason: reason,
        local: None,
    })
}

fn canonical_task_label(label: &TaskLabel) -> String {
    format!("{}:{}", label.package, label.name)
}
