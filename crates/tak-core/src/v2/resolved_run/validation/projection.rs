use std::collections::{BTreeMap, BTreeSet};

use crate::v2::{SessionReuse, TaskRuntime};

use super::super::{ResolvedRun, ResolvedRunError, ResolvedTaskUnit, ResourceRequest};

pub(super) fn validate(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    let tasks = run
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    validate_edges(run, &tasks)?;
    let mut sessions = BTreeMap::new();
    for job in &run.jobs {
        let units = job
            .task_ids
            .iter()
            .map(|id| tasks[id.as_str()])
            .collect::<Vec<_>>();
        if units.len() > 1
            && !matches!(
                job.session.as_ref().map(|session| &session.reuse),
                Some(SessionReuse::Container)
            )
        {
            return Err(ResolvedRunError::new(format!(
                "job `{}` owns multiple tasks without SessionReuse.Container",
                job.job_id
            )));
        }
        let pass_env_names = units
            .iter()
            .flat_map(|task| task.pass_env_names.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if units
            .iter()
            .any(|task| projected_resources(task) != job.resources)
        {
            return Err(ResolvedRunError::new(format!(
                "job `{}` resources do not match every task runtime",
                job.job_id
            )));
        }
        if units.len() > 1
            && units[1..]
                .iter()
                .any(|task| task.runtime != units[0].runtime)
        {
            return Err(ResolvedRunError::new(format!(
                "job `{}` fused task runtime definitions must be identical",
                job.job_id
            )));
        }
        if job.idempotent != units.iter().all(|task| task.idempotent)
            || job.pass_env_names != pass_env_names
            || units.iter().any(|task| task.affinity != job.affinity)
        {
            return Err(ResolvedRunError::new(format!(
                "job `{}` policy does not match its tasks",
                job.job_id
            )));
        }
        if let Some(session) = &job.session {
            session
                .validate()
                .map_err(|error| ResolvedRunError::new(error.to_string()))?;
            let effective = session
                .effective_affinity(job.affinity.as_ref())
                .map_err(|error| ResolvedRunError::new(error.to_string()))?;
            if effective != job.affinity {
                return Err(ResolvedRunError::new(format!(
                    "job `{}` did not inherit its session affinity",
                    job.job_id
                )));
            }
            if sessions
                .insert(session.id.as_str(), session)
                .is_some_and(|existing| existing != session)
            {
                return Err(ResolvedRunError::new(format!(
                    "session `{}` has conflicting definitions",
                    session.id
                )));
            }
        }
    }
    Ok(())
}

fn projected_resources(task: &ResolvedTaskUnit) -> ResourceRequest {
    task.runtime
        .as_ref()
        .and_then(TaskRuntime::resources)
        .map_or_else(ResourceRequest::default, |resources| ResourceRequest {
            cpu_millis: resources.cpu_millis,
            memory_bytes: resources.memory_bytes,
            ..ResourceRequest::default()
        })
}

fn validate_edges(
    run: &ResolvedRun,
    tasks: &BTreeMap<&str, &ResolvedTaskUnit>,
) -> Result<(), ResolvedRunError> {
    let expected = run
        .tasks
        .iter()
        .flat_map(|task| {
            task.dependencies.iter().filter_map(|dependency| {
                let dependency_job = &tasks[dependency.as_str()].job_id;
                (dependency_job != &task.job_id)
                    .then(|| (dependency_job.clone(), task.job_id.clone()))
            })
        })
        .collect::<BTreeSet<_>>();
    let actual = run
        .job_edges
        .iter()
        .map(|edge| {
            (
                edge.dependency_job_id.clone(),
                edge.dependent_job_id.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(ResolvedRunError::new(
            "job edges do not match task dependencies",
        ));
    }
    Ok(())
}
