use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::path::Path;

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use tak_core::v2::{
    JobContextManifest, JobEdge, RemoteRequirements, ResolvedJob, ResolvedRun, ResolvedRunOptions,
    ResolvedTaskUnit, ResourceRequest, RunSubmission, WorkspaceDescriptor,
};
use tak_loader::V2AuthoredRoot;

use super::RunCliArgs;
use super::overrides::ExecutionOverride;

mod context;
#[cfg(test)]
mod context_tests;
mod environment;
mod fusion;
#[cfg(test)]
mod fusion_tests;
mod graph;
#[cfg(test)]
mod graph_tests;
mod identity;
#[cfg(test)]
mod identity_tests;
mod placement;
mod scheduling;
mod sessions;
mod workspace_contexts;
use environment::{effective_env_names, environment_values};
use graph::{canonical, selected_tasks};
pub(super) use workspace_contexts::resolve as workspace_contexts;

pub(super) async fn resolve(
    root: &V2AuthoredRoot,
    args: &RunCliArgs,
    workspace: WorkspaceDescriptor,
    gitignored_paths: &BTreeSet<String>,
    socket_path: &Path,
    execution_override: Option<&ExecutionOverride>,
) -> Result<RunSubmission> {
    let selected = selected_tasks(&root.module, &args.labels)?;
    let job_ids = selected
        .iter()
        .enumerate()
        .map(|(index, task)| Ok((canonical(&task.name)?, format!("job-{index}"))))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let worktree_scope_key = identity::worktree_scope_key(root)?;
    let limiter_definitions = scheduling::limiters(&root.module, &selected, &worktree_scope_key)?;
    let queue_definitions = scheduling::queues(&root.module, &selected, &worktree_scope_key)?;
    let bindings = sessions::bindings(&root.module, &selected)?;
    let mut placement_cache = BTreeMap::<RemoteRequirements, Vec<_>>::new();
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
        let execution = execution_override
            .map(|override_| override_.execution(binding.execution.as_ref()))
            .transpose()?
            .or_else(|| binding.execution.clone());
        let session = if let Some(override_) = execution_override {
            let resolved = execution
                .as_ref()
                .ok_or_else(|| anyhow!("execution override did not resolve execution"))?;
            override_.session(binding.session.clone(), resolved)
        } else {
            binding.session.clone()
        };
        let effective_context = context::effective(task.context.as_ref(), session.as_ref());
        let context_paths =
            context::paths(&workspace.manifest, effective_context, gitignored_paths)?;
        let affinity = binding.affinity.clone();
        let job_id = job_ids[&task_id].clone();
        let runtime = execution
            .as_ref()
            .and_then(tak_core::v2::Execution::runtime)
            .cloned();
        let resources = runtime
            .as_ref()
            .and_then(tak_core::v2::TaskRuntime::resources)
            .map_or_else(ResourceRequest::default, |resources| ResourceRequest {
                cpu_millis: resources.cpu_millis,
                memory_bytes: resources.memory_bytes,
                execution_slots: NonZeroU32::MIN,
            });
        tasks.push(ResolvedTaskUnit {
            task_id: task_id.clone(),
            job_id: job_id.clone(),
            dependencies,
            steps: task.steps.clone(),
            outputs: task.outputs.clone(),
            pass_env_names: pass_env_names.clone(),
            idempotent: task.idempotent,
            affinity: affinity.clone(),
            timeout_s: task.timeout_s,
            runtime,
        });
        let (placement_policy, placement_candidates) =
            placement::resolve(execution.as_ref(), socket_path, &mut placement_cache).await?;
        let queue = scheduling::queue(&root.module, task)?;
        jobs.push(ResolvedJob {
            job_id,
            task_ids: vec![task_id],
            placement_policy,
            placement_candidates,
            resources,
            retry: scheduling::retry(&root.module, task),
            idempotent: task.idempotent,
            queue: queue.name,
            queue_slots: queue.slots,
            queue_priority: queue.priority,
            limiter_claims: scheduling::claims(task),
            affinity,
            session,
            context_manifest: JobContextManifest {
                paths: context_paths,
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

fn project_id(root: &V2AuthoredRoot) -> String {
    root.module.project_id.clone().unwrap_or_else(|| {
        let digest = format!(
            "{:x}",
            Sha256::digest(root.workspace_root.to_string_lossy().as_bytes())
        );
        format!("project-{}", &digest[..16])
    })
}
