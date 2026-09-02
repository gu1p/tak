use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use anyhow::{Result, anyhow};
use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_core::v2::{
    Affinity, JobContextManifest, JobEdge, OutputSelector, RemoteRequirements, ResolvedJob,
    ResolvedRun, ResolvedRunOptions, RunSubmission, Session, SessionReuse, WorkspaceDescriptor,
};

mod environment;
mod placement;
mod runtime;
mod task;

pub(super) async fn submission(
    spec: &WorkspaceSpec,
    targets: &[TaskLabel],
    max_parallel_jobs: usize,
    keep_going: bool,
    pass_env: &[String],
    workspace: WorkspaceDescriptor,
) -> Result<RunSubmission> {
    let (pass_env_names, environment_values) = environment::passed(pass_env)?;
    let session = shared_session(max_parallel_jobs)?;
    let affinity = session.affinity.clone();
    let target_ids = targets.iter().map(canonical).collect::<BTreeSet<_>>();
    let job_ids = spec
        .tasks
        .keys()
        .enumerate()
        .map(|(index, label)| (label.clone(), format!("job-{index}")))
        .collect::<BTreeMap<_, _>>();
    let context_paths = workspace
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    let mut candidate_cache = BTreeMap::<RemoteRequirements, Vec<_>>::new();
    let mut tasks = Vec::new();
    let mut jobs = Vec::new();
    for (label, authored) in &spec.tasks {
        let job_id = job_ids[label].clone();
        let mut unit = task::unit(authored, job_id.clone(), &pass_env_names)?;
        unit.affinity = affinity.clone();
        if target_ids.contains(&unit.task_id) {
            unit.outputs
                .push(OutputSelector::Glob { value: "**".into() });
        }
        let resolved_placement =
            placement::resolve(&authored.execution, &mut candidate_cache).await?;
        jobs.push(ResolvedJob {
            job_id,
            task_ids: vec![unit.task_id.clone()],
            placement_policy: resolved_placement.policy,
            placement_candidates: resolved_placement.candidates,
            resources: task::resources(authored)?,
            retry: task::retry(authored)?,
            idempotent: false,
            queue: None,
            queue_slots: NonZeroU32::MIN,
            queue_priority: 0,
            limiter_claims: Vec::new(),
            affinity: affinity.clone(),
            session: Some(session.clone()),
            context_manifest: JobContextManifest {
                paths: context_paths.clone(),
            },
            pass_env_names: pass_env_names.clone(),
        });
        tasks.push(unit);
    }
    let run = ResolvedRun {
        project_id: spec.project_id.clone(),
        targets: targets.iter().map(canonical).collect(),
        options: ResolvedRunOptions {
            max_parallel_jobs: NonZeroU32::new(u32::try_from(max_parallel_jobs)?)
                .ok_or_else(|| anyhow!("jobs must be positive"))?,
            keep_going,
        },
        workspace,
        job_edges: job_edges(spec, &job_ids),
        tasks,
        jobs,
        limiter_definitions: Vec::new(),
        queue_definitions: Vec::new(),
    };
    RunSubmission::new(
        format!("submit-{}", uuid::Uuid::new_v4()),
        run,
        environment_values,
    )
    .map_err(Into::into)
}

fn shared_session(max_parallel_jobs: usize) -> Result<Session> {
    let affinity = Affinity::require_same_node("tak-make")?;
    let parallelism = u32::try_from(max_parallel_jobs)?;
    Ok(Session::new(
        "tak-make",
        SessionReuse::shared_workspace(parallelism)?,
        Some(affinity),
    )?)
}

fn job_edges(spec: &WorkspaceSpec, job_ids: &BTreeMap<TaskLabel, String>) -> Vec<JobEdge> {
    spec.tasks
        .iter()
        .flat_map(|(label, task)| {
            task.deps
                .iter()
                .map(|dependency| (job_ids[dependency].clone(), job_ids[label].clone()))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(dependency_job_id, dependent_job_id)| JobEdge {
            dependency_job_id,
            dependent_job_id,
        })
        .collect()
}

fn canonical(label: &TaskLabel) -> String {
    if label.package == "//" {
        format!("//:{}", label.name)
    } else {
        format!("{}:{}", label.package, label.name)
    }
}
