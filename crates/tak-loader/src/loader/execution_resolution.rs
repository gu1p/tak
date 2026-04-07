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
        TaskExecutionDef::RemoteOnly { remote } => {
            Ok(TaskExecutionSpec::RemoteOnly(resolve_remote(remote)?))
        }
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
            if decision.remote.is_some() {
                bail!("execution ByCustomPolicy.decision local mode cannot include remote targets");
            }
            Ok(PolicyDecisionSpec::Local { reason })
        }
        PolicyDecisionModeDef::Remote => {
            let remote = decision.remote.ok_or_else(|| {
                anyhow!("execution ByCustomPolicy.decision remote mode requires remote")
            })?;
            let remote = resolve_remote(remote)?;
            Ok(PolicyDecisionSpec::Remote { reason, remote })
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
        pool,
        required_tags,
        required_capabilities,
        transport,
        runtime,
    } = remote;

    let pool = pool.map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    let required_tags = required_tags
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let required_capabilities = required_capabilities
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    let transport_kind = validate_remote_transport(transport)?;
    let runtime = validate_remote_runtime(runtime)?;

    Ok(RemoteSpec {
        pool,
        required_tags,
        required_capabilities,
        transport_kind,
        runtime,
    })
}
