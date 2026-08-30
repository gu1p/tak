use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{
    Execution, JobContextManifest, JobEdge, PlacementCandidate, PlacementKind, PlacementPolicy,
    RemoteSelection, ResolvedJob, ResolvedRun, ResolvedRunOptions, ResolvedTaskUnit,
    ResourceRequest, RunSubmission, WorkspaceDescriptor,
};
use tak_loader::V2AuthoredRoot;

use super::RunCliArgs;

mod environment;
mod fusion;
#[cfg(test)]
mod fusion_tests;
mod graph;
mod identity;
#[cfg(test)]
mod identity_tests;
mod scheduling;
mod sessions;
use environment::{effective_env_names, environment_values};
use graph::{canonical, selected_tasks};

pub(super) fn resolve(
    root: &V2AuthoredRoot,
    args: &RunCliArgs,
    workspace: WorkspaceDescriptor,
) -> Result<RunSubmission> {
    let selected = selected_tasks(&root.module, &args.labels)?;
    let job_ids = selected
        .iter()
        .enumerate()
        .map(|(index, task)| Ok((canonical(&task.name)?, format!("job-{index}"))))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let context_paths = workspace
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    let worktree_scope_key = identity::worktree_scope_key(root)?;
    let limiter_definitions = scheduling::limiters(&root.module, &selected, &worktree_scope_key)?;
    let queue_definitions = scheduling::queues(&root.module, &selected, &worktree_scope_key)?;
    let bindings = sessions::bindings(&root.module, &selected)?;
    let mut tasks = Vec::new();
    let mut jobs = Vec::new();
    for task in &selected {
        let task_id = canonical(&task.name)?;
        let dependencies = task
            .deps
            .iter()
            .map(|dependency| canonical(dependency))
            .collect::<Result<Vec<_>>>()?;
        let pass_env_names = effective_env_names(&root.module, task, &args.pass_env)?;
        let binding = &bindings[&task_id];
        let session = binding.session.clone();
        let affinity = binding.affinity.clone();
        let job_id = job_ids[&task_id].clone();
        tasks.push(ResolvedTaskUnit {
            task_id: task_id.clone(),
            job_id: job_id.clone(),
            dependencies,
            steps: task.steps.clone(),
            outputs: task.outputs.clone(),
            pass_env_names: pass_env_names.clone(),
            idempotent: task.idempotent,
            affinity: affinity.clone(),
        });
        jobs.push(ResolvedJob {
            job_id,
            task_ids: vec![task_id],
            placement_policy: PlacementPolicy {
                policy_id: "local".into(),
                selection: RemoteSelection::Sequential,
            },
            placement_candidates: placement_candidates(binding.execution.as_ref())?,
            resources: ResourceRequest::default(),
            retry: scheduling::retry(&root.module, task),
            idempotent: task.idempotent,
            queue: scheduling::queue_name(&root.module, task)?,
            limiter_claims: scheduling::claims(task),
            affinity,
            session,
            context_manifest: JobContextManifest {
                paths: context_paths.clone(),
            },
            pass_env_names,
        });
    }
    let fused = fusion::fuse_jobs(jobs)?;
    for task in &mut tasks {
        task.job_id = fused.job_ids[&task.job_id].clone();
    }
    let resolved_job_ids = tasks
        .iter()
        .map(|task| (task.task_id.clone(), task.job_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let job_edges = tasks
        .iter()
        .flat_map(|task| {
            task.dependencies.iter().filter_map(|dependency| {
                let dependency_job_id = resolved_job_ids[dependency].clone();
                (dependency_job_id != task.job_id).then(|| (dependency_job_id, task.job_id.clone()))
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|(dependency_job_id, dependent_job_id)| JobEdge {
            dependency_job_id,
            dependent_job_id,
        })
        .collect();
    let run = ResolvedRun {
        project_id: project_id(root),
        targets: args
            .labels
            .iter()
            .map(|label| canonical(label))
            .collect::<Result<Vec<_>>>()?,
        options: ResolvedRunOptions {
            max_parallel_jobs: NonZeroU32::new(u32::try_from(args.jobs)?)
                .ok_or_else(|| anyhow!("jobs must be positive"))?,
            keep_going: args.keep_going,
        },
        workspace,
        tasks,
        jobs: fused.jobs,
        job_edges,
        limiter_definitions,
        queue_definitions,
    };
    let environment_values = environment_values(&run)?;
    Ok(RunSubmission::new(
        format!("submit-{}", uuid::Uuid::new_v4()),
        run,
        environment_values,
    )?)
}

fn placement_candidates(execution: Option<&Execution>) -> Result<Vec<PlacementCandidate>> {
    match execution {
        None | Some(Execution::LocalOnly { .. }) => Ok(vec![PlacementCandidate {
            node_id: "local".into(),
            kind: PlacementKind::Local,
            transport: None,
            reason: "local execution".into(),
        }]),
        Some(Execution::RemoteOnly { .. }) => {
            bail!("takd remote placement candidate resolution is not active in this build")
        }
    }
}

fn project_id(root: &V2AuthoredRoot) -> String {
    root.module.project_id.clone().unwrap_or_else(|| {
        let digest = format!(
            "{:x}",
            Sha256::digest(root.workspace_root.to_string_lossy().as_bytes())
        );
        format!("project-{}", &digest[..16])
    })
}
