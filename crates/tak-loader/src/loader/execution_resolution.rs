use anyhow::{Result, anyhow, bail};
use tak_core::model::{
    LocalDef, LocalSpec, PolicyDecisionDef, PolicyDecisionModeDef, PolicyDecisionSpec, RemoteDef,
    RemoteSelectionDef, RemoteSelectionSpec, RemoteSpec, TaskExecutionDef, TaskExecutionSpec,
};

use super::remote_validation::{
    validate_local_runtime, validate_remote_transport, validate_runtime,
};

pub(crate) fn resolve_execution(
    execution: TaskExecutionDef,
    package: &str,
) -> Result<TaskExecutionSpec> {
    match execution {
        TaskExecutionDef::LocalOnly { local } => {
            let id = local.id.trim().to_string();
            if id.is_empty() {
                bail!("execution LocalOnly.local.id cannot be empty");
            }
            if local.max_parallel_tasks == 0 {
                bail!("execution LocalOnly.local.max_parallel_tasks must be >= 1");
            }
            let runtime = validate_local_runtime(local.runtime, package, "Execution.Local")?;
            Ok(TaskExecutionSpec::LocalOnly(LocalSpec {
                id,
                max_parallel_tasks: local.max_parallel_tasks,
                runtime,
            }))
        }
        TaskExecutionDef::RemoteOnly { remote } => Ok(TaskExecutionSpec::RemoteOnly(
            resolve_remote(remote, package)?,
        )),
        TaskExecutionDef::ByCustomPolicy {
            policy_name,
            decision,
        } => {
            let policy_name = policy_name.trim().to_string();
            if policy_name.is_empty() {
                bail!("execution ByCustomPolicy.policy_name cannot be empty");
            }
            let decision = decision
                .map(|decision| resolve_policy_decision(*decision, package))
                .transpose()?;
            Ok(TaskExecutionSpec::ByCustomPolicy {
                policy_name,
                decision,
            })
        }
        TaskExecutionDef::UseSession { name, cascade } => {
            let name = name.trim().to_string();
            if name.is_empty() {
                bail!("execution UseSession.name cannot be empty");
            }
            Ok(TaskExecutionSpec::UseSession { name, cascade })
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
pub(crate) fn resolve_policy_decision(
    decision: PolicyDecisionDef,
    package: &str,
) -> Result<PolicyDecisionSpec> {
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
            let local = decision
                .local
                .map(|local| {
                    resolve_local(local, package, "execution ByCustomPolicy.decision.local")
                })
                .transpose()?;
            Ok(PolicyDecisionSpec::Local { reason, local })
        }
        PolicyDecisionModeDef::Remote => {
            let remote = decision.remote.ok_or_else(|| {
                anyhow!("execution ByCustomPolicy.decision remote mode requires remote")
            })?;
            if decision.local.is_some() {
                bail!("execution ByCustomPolicy.decision remote mode cannot include local targets");
            }
            let remote = resolve_remote(remote, package)?;
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
fn resolve_local(local: LocalDef, package: &str, field: &str) -> Result<LocalSpec> {
    let id = local.id.trim().to_string();
    if id.is_empty() {
        bail!("{field}.id cannot be empty");
    }
    if local.max_parallel_tasks == 0 {
        bail!("{field}.max_parallel_tasks must be >= 1");
    }
    let runtime = validate_local_runtime(local.runtime, package, field)?;

    Ok(LocalSpec {
        id,
        max_parallel_tasks: local.max_parallel_tasks,
        runtime,
    })
}

fn resolve_remote(remote: RemoteDef, package: &str) -> Result<RemoteSpec> {
    let RemoteDef {
        pool,
        required_tags,
        required_capabilities,
        transport,
        runtime,
        selection,
    } = remote;

    let pool = pool
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
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
    let runtime = validate_runtime(runtime, package, "Remote")?;

    Ok(RemoteSpec {
        pool,
        required_tags,
        required_capabilities,
        transport_kind,
        runtime,
        selection: resolve_remote_selection(selection),
    })
}

fn resolve_remote_selection(selection: RemoteSelectionDef) -> RemoteSelectionSpec {
    match selection {
        RemoteSelectionDef::Sequential => RemoteSelectionSpec::Sequential,
        RemoteSelectionDef::Shuffle => RemoteSelectionSpec::Shuffle,
    }
}
