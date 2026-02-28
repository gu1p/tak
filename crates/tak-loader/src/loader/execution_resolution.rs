fn resolve_execution(execution: TaskExecutionDef) -> Result<TaskExecutionSpec> {
    match execution {
        TaskExecutionDef::LocalOnly { local } => {
            let id = local.id.trim().to_string();
            if id.is_empty() {
                bail!("execution LocalOnly.local.id cannot be empty");
            }
            if local.max_parallel_tasks == 0 {
                bail!("execution LocalOnly.local.max_parallel_tasks must be >= 1");
            }
            Ok(TaskExecutionSpec::LocalOnly(LocalSpec {
                id,
                max_parallel_tasks: local.max_parallel_tasks,
            }))
        }
        TaskExecutionDef::RemoteOnly { remote } => Ok(TaskExecutionSpec::RemoteOnly(
            resolve_remote_selection(remote)?,
        )),
        TaskExecutionDef::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let policy_name = policy_name.trim().to_string();
            if policy_name.is_empty() {
                bail!("execution ByCustomPolicy.policy_name cannot be empty");
            }
            let decision = decision.map(resolve_policy_decision).transpose()?;
            Ok(TaskExecutionSpec::ByCustomPolicy {
                policy_name,
                decision,
            })
        }
    }
}

/// Resolves a policy-produced execution decision into strict runtime form.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_policy_decision(decision: PolicyDecisionDef) -> Result<PolicyDecisionSpec> {
    let reason = {
        let normalized = decision.reason.trim().to_string();
        if normalized.is_empty() {
            "DEFAULT_LOCAL_POLICY".to_string()
        } else {
            normalized
        }
    };

    match decision.mode {
        PolicyDecisionModeDef::Local => {
            if decision.remote.is_some() || !decision.remotes.is_empty() {
                bail!("execution ByCustomPolicy.decision local mode cannot include remote targets");
            }
            Ok(PolicyDecisionSpec::Local { reason })
        }
        PolicyDecisionModeDef::Remote => {
            if !decision.remotes.is_empty() {
                bail!("execution ByCustomPolicy.decision remote mode cannot include remotes list");
            }
            let remote = decision.remote.ok_or_else(|| {
                anyhow!("execution ByCustomPolicy.decision remote mode requires remote")
            })?;
            let remote = resolve_remote(*remote)?;
            if remote.endpoint.is_none() {
                bail!(
                    "execution ByCustomPolicy.decision remote target {} requires endpoint",
                    remote.id
                );
            }
            Ok(PolicyDecisionSpec::Remote { reason, remote })
        }
        PolicyDecisionModeDef::RemoteAny => {
            if decision.remote.is_some() {
                bail!(
                    "execution ByCustomPolicy.decision remote_any mode cannot include singular remote"
                );
            }
            if decision.remotes.is_empty() {
                bail!(
                    "execution ByCustomPolicy.decision remote_any mode requires non-empty remotes"
                );
            }

            let mut remotes = Vec::with_capacity(decision.remotes.len());
            for remote in decision.remotes {
                let resolved = resolve_remote(remote)?;
                if resolved.endpoint.is_none() {
                    bail!(
                        "execution ByCustomPolicy.decision remote_any target {} requires endpoint",
                        resolved.id
                    );
                }
                remotes.push(resolved);
            }

            Ok(PolicyDecisionSpec::RemoteAny { reason, remotes })
        }
    }
}

/// Resolves one remote selection shape while enforcing non-empty node ids.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_remote_selection(selection: RemoteSelectionDef) -> Result<RemoteSelectionSpec> {
    match selection {
        RemoteSelectionDef::Single(remote) => {
            Ok(RemoteSelectionSpec::Single(resolve_remote(*remote)?))
        }
        RemoteSelectionDef::List(remotes) => {
            if remotes.is_empty() {
                bail!("execution RemoteOnly.remote list cannot be empty");
            }
            let mut resolved = Vec::with_capacity(remotes.len());
            for remote in remotes {
                resolved.push(resolve_remote(remote)?);
            }
            Ok(RemoteSelectionSpec::List(resolved))
        }
    }
}

/// Resolves one remote node declaration used by task execution selectors.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn resolve_remote(remote: RemoteDef) -> Result<RemoteSpec> {
    let RemoteDef {
        id: raw_id,
        endpoint: raw_endpoint,
        transport,
        workspace,
        result,
        runtime,
    } = remote;

    let id = raw_id.trim().to_string();
    if id.is_empty() {
        bail!("execution Remote.id cannot be empty");
    }
    let endpoint = raw_endpoint.and_then(|value| {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    });

    let (transport_kind, service_auth_env) = validate_remote_transport(transport)?;
    validate_remote_workspace(workspace)?;
    validate_remote_result(result)?;
    let runtime = validate_remote_runtime(runtime)?;

    Ok(RemoteSpec {
        id,
        endpoint,
        transport_kind,
        service_auth_env,
        runtime,
    })
}
