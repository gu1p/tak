use std::collections::BTreeSet;

use anyhow::{Result, bail};
use tak_core::v2::{AuthoredModule, AuthoredTask, EnvironmentValue, ResolvedRun};

pub(super) fn effective_env_names(
    module: &AuthoredModule,
    task: &AuthoredTask,
    cli_names: &[String],
) -> Result<Vec<String>> {
    let names = module
        .defaults
        .pass_env
        .as_strs()
        .into_iter()
        .chain(task.pass_env.as_strs())
        .chain(cli_names.iter().map(String::as_str));
    Ok(tak_core::v2::PassEnv::new(names)?
        .as_strs()
        .into_iter()
        .map(str::to_owned)
        .collect())
}

pub(super) fn environment_values(run: &ResolvedRun) -> Result<Vec<EnvironmentValue>> {
    let names = run
        .jobs
        .iter()
        .flat_map(|job| job.pass_env_names.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut missing = Vec::new();
    let mut values = Vec::new();
    for name in names {
        match std::env::var(&name) {
            Ok(value) => values.push(EnvironmentValue::new(name, value)?),
            Err(_) => missing.push(name),
        }
    }
    if !missing.is_empty() {
        bail!(
            "missing requested environment variables: {}",
            missing.join(", ")
        );
    }
    Ok(values)
}
