use std::path::Path;

use anyhow::{Result, bail};
use tak_core::model::{ExecutionPolicyDef, TaskExecutionDef, TaskExecutionSpec};

use super::{
    MergeState,
    execution_policy_resolution::{
        normalize_execution_policy_name, resolve_execution_policy,
        resolve_execution_policy_reference,
    },
    execution_resolution::resolve_execution,
    global_config::{GlobalExecutionConfig, validate_global_execution_policy},
};

pub(crate) fn register_global_execution_policies(
    config: GlobalExecutionConfig,
    state: &mut MergeState,
) -> Result<()> {
    state.default_execution_policy = config
        .default_execution_policy
        .map(|name| normalize_execution_policy_name(&name))
        .transpose()?;
    for policy in config.execution_policies {
        let resolved = resolve_execution_policy(policy, "//")?;
        validate_global_execution_policy(&resolved)?;
        if state.execution_policies.contains_key(&resolved.name) {
            bail!(
                "duplicate global execution_policy definition: {}",
                resolved.name
            );
        }
        state
            .global_execution_policy_names
            .insert(resolved.name.clone());
        state.execution_policy_origins.insert(
            resolved.name.clone(),
            Path::new("$XDG_CONFIG_HOME/tak/config.toml").into(),
        );
        state
            .execution_policies
            .insert(resolved.name.clone(), resolved);
    }
    Ok(())
}

pub(crate) fn register_module_execution_policies(
    module_path: &Path,
    package: &str,
    policies: &[ExecutionPolicyDef],
    state: &mut MergeState,
) -> Result<()> {
    for policy in policies {
        let resolved = resolve_execution_policy(policy.clone(), package)?;
        let is_global_override = state.global_execution_policy_names.remove(&resolved.name);
        if !is_global_override
            && let Some(previous) = state.execution_policy_origins.get(&resolved.name)
        {
            bail!(
                "duplicate execution_policy definition: {}\nfirst defined in {}\nconflicts with {}",
                resolved.name,
                previous.display(),
                module_path.display()
            );
        }
        state
            .execution_policy_origins
            .insert(resolved.name.clone(), module_path.to_path_buf());
        state
            .execution_policies
            .insert(resolved.name.clone(), resolved);
    }
    Ok(())
}

pub(crate) fn resolve_task_execution(
    task_name: &str,
    execution: Option<TaskExecutionDef>,
    execution_policy: Option<String>,
    module_default_policy: Option<&str>,
    global_default_policy: Option<&str>,
    package: &str,
    state: &MergeState,
) -> Result<TaskExecutionSpec> {
    if execution.is_some() && execution_policy.is_some() {
        bail!("task `{task_name}` cannot set both execution and execution_policy");
    }
    if let Some(execution) = execution {
        return resolve_execution(execution, package);
    }
    if let Some(policy_name) = execution_policy {
        return resolve_execution_policy_reference(&policy_name, &state.execution_policies);
    }
    if let Some(policy_name) = module_default_policy {
        return resolve_execution_policy_reference(policy_name, &state.execution_policies);
    }
    if let Some(policy_name) = global_default_policy {
        return resolve_execution_policy_reference(policy_name, &state.execution_policies);
    }
    Ok(TaskExecutionSpec::default())
}
