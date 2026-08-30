use std::collections::{BTreeMap, BTreeSet};

use super::super::{ResolvedRun, ResolvedRunError, ResolvedTaskUnit};

pub(super) fn validate(run: &ResolvedRun) -> Result<(), ResolvedRunError> {
    let tasks = run
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect::<BTreeMap<_, _>>();
    validate_edges(run, &tasks)?;
    for job in &run.jobs {
        let units = job
            .task_ids
            .iter()
            .map(|id| tasks[id.as_str()])
            .collect::<Vec<_>>();
        let pass_env_names = units
            .iter()
            .flat_map(|task| task.pass_env_names.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
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
        }
    }
    Ok(())
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
