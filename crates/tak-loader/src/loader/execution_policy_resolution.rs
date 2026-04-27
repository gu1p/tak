use anyhow::{Result, anyhow, bail};
use tak_core::model::{
    ExecutionPlacementSpec, ExecutionPolicyDef, ExecutionPolicySpec, TaskExecutionDef,
    TaskExecutionSpec,
};

use super::execution_resolution::resolve_execution;

pub(crate) fn resolve_execution_policy(
    policy: ExecutionPolicyDef,
    package: &str,
) -> Result<ExecutionPolicySpec> {
    let name = normalize_execution_policy_name(&policy.name)?;
    if policy.placements.is_empty() {
        bail!("execution_policy `{name}` requires at least one placement");
    }
    let placements = policy
        .placements
        .into_iter()
        .map(|placement| resolve_policy_placement(&name, placement, package))
        .collect::<Result<Vec<_>>>()?;

    Ok(ExecutionPolicySpec {
        name,
        placements,
        doc: policy.doc,
    })
}

pub(crate) fn resolve_execution_policy_reference(
    name: &str,
    policies: &std::collections::BTreeMap<String, ExecutionPolicySpec>,
) -> Result<TaskExecutionSpec> {
    let name = normalize_execution_policy_name(name)?;
    let policy = policies
        .get(&name)
        .ok_or_else(|| anyhow!("unknown execution_policy `{name}`"))?;
    Ok(TaskExecutionSpec::ByExecutionPolicy {
        name,
        placements: policy.placements.clone(),
    })
}

pub(crate) fn normalize_execution_policy_name(name: &str) -> Result<String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        bail!("execution_policy name cannot be empty");
    }
    Ok(name)
}

fn resolve_policy_placement(
    policy_name: &str,
    placement: TaskExecutionDef,
    package: &str,
) -> Result<ExecutionPlacementSpec> {
    match resolve_execution(placement, package)? {
        TaskExecutionSpec::LocalOnly(local) => Ok(ExecutionPlacementSpec::Local(local)),
        TaskExecutionSpec::RemoteOnly(remote) => Ok(ExecutionPlacementSpec::Remote(remote)),
        TaskExecutionSpec::ByCustomPolicy { .. } => {
            bail!("execution_policy `{policy_name}` placements cannot use Execution.Policy")
        }
        TaskExecutionSpec::UseSession { .. } => {
            bail!("execution_policy `{policy_name}` placements cannot use Execution.Session")
        }
        TaskExecutionSpec::ByExecutionPolicy { .. } => {
            bail!("execution_policy `{policy_name}` placements cannot use execution_policy")
        }
    }
}
