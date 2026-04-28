use std::path::Path;

use anyhow::{Result, bail};
use tak_core::model::{TaskExecutionDef, TaskExecutionSpec};

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

pub(crate) fn resolve_task_execution(
    execution: Option<TaskExecutionDef>,
    module_default_execution: Option<TaskExecutionDef>,
    global_default_policy: Option<&str>,
    package: &str,
    state: &MergeState,
) -> Result<TaskExecutionSpec> {
    if let Some(execution) = execution {
        return resolve_execution(execution, package);
    }
    if let Some(execution) = module_default_execution {
        return resolve_execution(execution, package);
    }
    if let Some(policy_name) = global_default_policy {
        return resolve_execution_policy_reference(policy_name, &state.execution_policies);
    }
    Ok(TaskExecutionSpec::default())
}
