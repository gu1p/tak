use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use tak_core::v2::{PassEnv, ResolvedTaskUnit};

use super::{WorkerAttemptIdentity, WorkerAttemptPayload};

mod paths;

pub(super) fn validate(
    identity: &WorkerAttemptIdentity,
    payload: &WorkerAttemptPayload,
) -> Result<()> {
    let environment = canonical_names(
        payload
            .environment_values
            .iter()
            .map(|value| value.name.as_str()),
    )?;
    let workspace = payload
        .workspace
        .descriptor
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    validate_context(&payload.context_manifest.paths, &workspace)?;
    let mut tasks = BTreeSet::new();
    if payload.tasks.is_empty() {
        bail!("worker dispatch requires tasks");
    }
    for task in &payload.tasks {
        validate_task(identity, task, &environment)?;
        if !tasks.insert(task.task_id.as_str()) {
            bail!("worker dispatch contains duplicate task ids");
        }
    }
    Ok(())
}

fn validate_task(
    identity: &WorkerAttemptIdentity,
    task: &ResolvedTaskUnit,
    environment: &BTreeSet<&str>,
) -> Result<()> {
    if task.job_id != identity.job_id || !identifier(&task.task_id) {
        bail!("worker dispatch tasks do not match the job identity");
    }
    let passed = canonical_names(task.pass_env_names.iter().map(String::as_str))?;
    if !passed.is_subset(environment) {
        bail!("worker dispatch passed environment is incomplete");
    }
    paths::validate(task)
}

fn canonical_names<'a>(names: impl Iterator<Item = &'a str>) -> Result<BTreeSet<&'a str>> {
    let names = names.collect::<Vec<_>>();
    let canonical = PassEnv::new(&names)?;
    if canonical.as_strs() != names {
        bail!("worker environment names must be sorted and unique");
    }
    Ok(names.into_iter().collect())
}

fn validate_context(paths: &[String], workspace: &BTreeSet<&str>) -> Result<()> {
    let context = paths.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if context.len() != paths.len() || context.iter().copied().ne(paths.iter().map(String::as_str))
    {
        bail!("worker context paths must be sorted and unique");
    }
    for path in &context {
        if !workspace.contains(path) {
            bail!("worker context path is missing from the workspace manifest");
        }
        for ancestor in Path::new(path).ancestors().skip(1).filter_map(Path::to_str) {
            if !ancestor.is_empty() && workspace.contains(ancestor) && !context.contains(ancestor) {
                bail!("worker context manifest is not ancestor-closed");
            }
        }
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}
