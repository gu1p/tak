use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tak_core::model::{
    ContainerRuntimeSourceSpec, ExecutionPlacementSpec, ExecutionPolicyDef, ExecutionPolicySpec,
};

#[derive(Debug, Default)]
pub(crate) struct GlobalExecutionConfig {
    pub(crate) default_execution_policy: Option<String>,
    pub(crate) execution_policies: Vec<ExecutionPolicyDef>,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    defaults: ConfigDefaults,
    #[serde(default)]
    default_execution_policy: Option<String>,
    #[serde(default)]
    execution_policies: Vec<ExecutionPolicyDef>,
}

#[derive(Debug, Default, Deserialize)]
struct ConfigDefaults {
    #[serde(default)]
    execution_policy: Option<String>,
}

pub(crate) fn load_global_execution_config() -> Result<GlobalExecutionConfig> {
    let Some(path) = config_path()? else {
        return Ok(GlobalExecutionConfig::default());
    };
    if !path.exists() {
        return Ok(GlobalExecutionConfig::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read Tak config {}", path.display()))?;
    let parsed: ConfigFile = toml::from_str(&raw)
        .with_context(|| format!("failed to parse Tak config {}", path.display()))?;
    Ok(GlobalExecutionConfig {
        default_execution_policy: parsed
            .defaults
            .execution_policy
            .or(parsed.default_execution_policy),
        execution_policies: parsed.execution_policies,
    })
}

pub(crate) fn validate_global_execution_policy(policy: &ExecutionPolicySpec) -> Result<()> {
    for placement in &policy.placements {
        let source = match placement {
            ExecutionPlacementSpec::Local(local) => local.runtime.as_ref(),
            ExecutionPlacementSpec::Remote(remote) => remote.runtime.as_ref(),
        };
        if source.is_some_and(|runtime| {
            matches!(
                runtime,
                tak_core::model::RemoteRuntimeSpec::Containerized {
                    source: ContainerRuntimeSourceSpec::Dockerfile { .. }
                }
            )
        }) {
            bail!(
                "global execution_policy `{}` cannot use Runtime.Dockerfile; define Dockerfile runtimes in TASKS.py",
                policy.name
            );
        }
    }
    Ok(())
}

fn config_path() -> Result<Option<PathBuf>> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(Some(PathBuf::from(root).join("tak").join("config.toml")));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(Some(
            PathBuf::from(home)
                .join(".config")
                .join("tak")
                .join("config.toml"),
        ));
    }
    Ok(None)
}
