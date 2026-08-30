use std::collections::{BTreeMap, BTreeSet};

use super::{ResolvedRun, ResolvedRunError, validate_digest};
use crate::v2::PassEnv;

mod graph;
mod projection;
mod scheduling;

pub(super) fn validate(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    validate_identifier("project id", &run.project_id)?;
    run.workspace.manifest.validate()?;
    validate_digest(&run.workspace.archive_sha256)?;
    let tasks = unique_ids("task", run.tasks.iter().map(|task| task.task_id.as_str()))?;
    let jobs = unique_ids("job", run.jobs.iter().map(|job| job.job_id.as_str()))?;
    if run.targets.is_empty() {
        return Err(ResolvedRunError::new("run requires at least one target"));
    }
    require_references("target", run.targets.iter(), &tasks)?;
    validate_tasks(run, &tasks, &jobs)?;
    graph::task_graph(run)?;
    validate_jobs(run, &tasks)?;
    scheduling::validate(run)?;
    projection::validate(run)?;
    graph::job_graph(run, &jobs)?;
    validate_definitions(run)?;
    Ok(())
}

pub(super) fn validate_identifier(kind: &str, value: &str) -> Result<(), ResolvedRunError> {
    if !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control) {
        return Ok(());
    }
    Err(ResolvedRunError::new(format!("invalid {kind} `{value}`")))
}

fn unique_ids<'a>(
    kind: &str,
    values: impl Iterator<Item = &'a str>,
) -> Result<BTreeSet<String>, ResolvedRunError> {
    let mut result = BTreeSet::new();
    for value in values {
        validate_identifier(kind, value)?;
        if !result.insert(value.to_owned()) {
            return Err(ResolvedRunError::new(format!("duplicate {kind} `{value}`")));
        }
    }
    Ok(result)
}

fn require_references<'a>(
    kind: &str,
    values: impl Iterator<Item = &'a String>,
    known: &BTreeSet<String>,
) -> Result<(), ResolvedRunError> {
    for value in values {
        if !known.contains(value) {
            return Err(ResolvedRunError::new(format!("unknown {kind} `{value}`")));
        }
    }
    Ok(())
}

fn validate_tasks(
    run: &ResolvedRun,
    tasks: &BTreeSet<String>,
    jobs: &BTreeSet<String>,
) -> Result<(), ResolvedRunError> {
    for task in &run.tasks {
        if !jobs.contains(&task.job_id) {
            return Err(ResolvedRunError::new(format!(
                "unknown job `{}`",
                task.job_id
            )));
        }
        require_references("task dependency", task.dependencies.iter(), tasks)?;
        validate_names(&task.pass_env_names)?;
        if let Some(affinity) = &task.affinity {
            affinity
                .validate()
                .map_err(|error| ResolvedRunError::new(error.to_string()))?;
        }
    }
    Ok(())
}

fn validate_jobs(run: &ResolvedRun, tasks: &BTreeSet<String>) -> Result<(), ResolvedRunError> {
    let paths = run
        .workspace
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut membership = BTreeMap::new();
    for job in &run.jobs {
        if job.task_ids.is_empty() || job.placement_candidates.is_empty() {
            return Err(ResolvedRunError::new(format!(
                "job `{}` is incomplete",
                job.job_id
            )));
        }
        require_references("job task", job.task_ids.iter(), tasks)?;
        validate_names(&job.pass_env_names)?;
        if let Some(affinity) = &job.affinity {
            affinity
                .validate()
                .map_err(|error| ResolvedRunError::new(error.to_string()))?;
        }
        for path in &job.context_manifest.paths {
            if !paths.contains(path.as_str()) {
                return Err(ResolvedRunError::new(format!(
                    "unknown context path `{path}`"
                )));
            }
        }
        for task in &job.task_ids {
            if membership.insert(task, &job.job_id).is_some() {
                return Err(ResolvedRunError::new(format!(
                    "task `{task}` belongs to multiple jobs"
                )));
            }
        }
    }
    for task in &run.tasks {
        if membership.get(&task.task_id) != Some(&&task.job_id) {
            return Err(ResolvedRunError::new(format!(
                "task `{}` job membership mismatch",
                task.task_id
            )));
        }
    }
    Ok(())
}

fn validate_names(names: &[String]) -> Result<(), ResolvedRunError> {
    let canonical =
        PassEnv::new(names).map_err(|error| ResolvedRunError::new(error.to_string()))?;
    if canonical.as_strs() != names.iter().map(String::as_str).collect::<Vec<_>>() {
        return Err(ResolvedRunError::new(
            "environment names must be sorted and unique",
        ));
    }
    Ok(())
}

fn validate_definitions(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    let queues = unique_ids(
        "queue",
        run.queue_definitions.iter().map(|item| item.name.as_str()),
    )?;
    let limiters = unique_ids(
        "limiter",
        run.limiter_definitions.iter().map(|item| item.name()),
    )?;
    let limiter_definitions = run
        .limiter_definitions
        .iter()
        .map(|definition| (definition.name(), definition))
        .collect::<BTreeMap<_, _>>();
    for job in &run.jobs {
        if let Some(queue) = &job.queue
            && !queues.contains(queue)
        {
            return Err(ResolvedRunError::new(format!("unknown queue `{queue}`")));
        }
        let mut claims = BTreeSet::new();
        for claim in &job.limiter_claims {
            if !limiters.contains(&claim.name) {
                return Err(ResolvedRunError::new(format!(
                    "unknown limiter `{}`",
                    claim.name
                )));
            }
            if !claims.insert(claim.name.as_str()) {
                return Err(ResolvedRunError::new(format!(
                    "job `{}` has duplicate limiter claim `{}`",
                    job.job_id, claim.name
                )));
            }
            let definition = limiter_definitions[claim.name.as_str()];
            let capacity = definition.capacity_millis();
            if claim.amount_millis.get() > capacity {
                return Err(ResolvedRunError::new(format!(
                    "job `{}` limiter claim `{}` exceeds capacity",
                    job.job_id, claim.name
                )));
            }
            if matches!(definition, super::LimiterDefinition::Lock { .. })
                && claim.amount_millis.get() != capacity
            {
                return Err(ResolvedRunError::new(format!(
                    "job `{}` lock claim `{}` must acquire the whole lock",
                    job.job_id, claim.name
                )));
            }
        }
    }
    Ok(())
}
