use anyhow::{Result, bail};
use tak_core::v2::{EnvironmentValue, PassEnv};

pub(super) fn passed(names: &[String]) -> Result<(Vec<String>, Vec<EnvironmentValue>)> {
    let names = PassEnv::new(names)?
        .as_strs()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut missing = Vec::new();
    let mut values = Vec::new();
    for name in &names {
        match std::env::var(name) {
            Ok(value) => values.push(EnvironmentValue::new(name, value)?),
            Err(_) => missing.push(name.as_str()),
        }
    }
    if !missing.is_empty() {
        bail!(
            "missing requested environment variables: {}",
            missing.join(", ")
        );
    }
    Ok((names, values))
}
