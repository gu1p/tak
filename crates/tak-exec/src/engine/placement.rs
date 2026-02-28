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
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(remote)) => {
            let endpoint = remote_endpoint_for_strict(
                remote,
                "strict pin execution",
                &task.label.to_string(),
            )?;
            Ok(TaskPlacement {
                placement_mode: PlacementMode::Remote,
                remote_node_id: Some(remote.id.clone()),
                strict_remote_target: Some(StrictRemoteTarget {
                    node_id: remote.id.clone(),
                    endpoint,
                    transport_kind: remote.transport_kind,
                    service_auth_env: remote.service_auth_env.clone(),
                    runtime: remote.runtime.clone(),
                }),
                ordered_remote_targets: Vec::new(),
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::List(remotes)) => {
            if remotes.is_empty() {
                bail!(
                    "infra error: task {} has no remote fallback candidates",
                    task.label
                );
            }

            let mut ordered_remote_targets = Vec::with_capacity(remotes.len());
            for remote in remotes {
                let endpoint = remote_endpoint_for_strict(
                    remote,
                    "fallback execution",
                    &task.label.to_string(),
                )?;
                ordered_remote_targets.push(StrictRemoteTarget {
                    node_id: remote.id.clone(),
                    endpoint,
                    transport_kind: remote.transport_kind,
                    service_auth_env: remote.service_auth_env.clone(),
                    runtime: remote.runtime.clone(),
                });
            }

            Ok(TaskPlacement {
                placement_mode: PlacementMode::Remote,
                remote_node_id: None,
                strict_remote_target: None,
                ordered_remote_targets,
                remote_protocol_mode: None,
                decision_reason: None,
            })
        }
        TaskExecutionSpec::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let resolved_decision = if let Some(decision) = decision.as_ref() {
                decision.clone()
            } else {
                let tasks_file = tasks_file_for_label(workspace_root, &task.label);
                evaluate_named_policy_decision(&tasks_file, policy_name).with_context(|| {
                    format!(
                        "runtime policy evaluation failed for task {} (policy={policy_name})",
                        task.label
                    )
                })?
            };
            match &resolved_decision {
                PolicyDecisionSpec::Local { reason } => Ok(TaskPlacement {
                    placement_mode: PlacementMode::Local,
                    remote_node_id: None,
                    strict_remote_target: None,
                    ordered_remote_targets: Vec::new(),
                    remote_protocol_mode: None,
                    decision_reason: Some(reason.clone()),
                }),
                PolicyDecisionSpec::Remote { reason, remote } => {
                    let endpoint = remote_endpoint_for_strict(
                        remote,
                        "policy strict remote execution",
                        &task.label.to_string(),
                    )?;
                    Ok(TaskPlacement {
                        placement_mode: PlacementMode::Remote,
                        remote_node_id: Some(remote.id.clone()),
                        strict_remote_target: Some(StrictRemoteTarget {
                            node_id: remote.id.clone(),
                            endpoint,
                            transport_kind: remote.transport_kind,
                            service_auth_env: remote.service_auth_env.clone(),
                            runtime: remote.runtime.clone(),
                        }),
                        ordered_remote_targets: Vec::new(),
                        remote_protocol_mode: None,
                        decision_reason: Some(reason.clone()),
                    })
                }
                PolicyDecisionSpec::RemoteAny { reason, remotes } => {
                    if remotes.is_empty() {
                        bail!(
                            "infra error: policy decision for task {} has no remote fallback candidates",
                            task.label
                        );
                    }

                    let mut ordered_remote_targets = Vec::with_capacity(remotes.len());
                    for remote in remotes {
                        let endpoint = remote_endpoint_for_strict(
                            remote,
                            "policy fallback execution",
                            &task.label.to_string(),
                        )?;
                        ordered_remote_targets.push(StrictRemoteTarget {
                            node_id: remote.id.clone(),
                            endpoint,
                            transport_kind: remote.transport_kind,
                            service_auth_env: remote.service_auth_env.clone(),
                            runtime: remote.runtime.clone(),
                        });
                    }

                    Ok(TaskPlacement {
                        placement_mode: PlacementMode::Remote,
                        remote_node_id: None,
                        strict_remote_target: None,
                        ordered_remote_targets,
                        remote_protocol_mode: None,
                        decision_reason: Some(reason.clone()),
                    })
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

/// Resolves a strict remote endpoint value or returns a contextual infra error.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn remote_endpoint_for_strict(remote: &RemoteSpec, mode: &str, task_label: &str) -> Result<String> {
    remote.endpoint.clone().ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} is missing endpoint for {mode} in task {task_label}",
            remote.id
        )
    })
}
