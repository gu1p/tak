use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha256};
use tak_core::v2::{
    Affinity, AuthoredModule, AuthoredTask, EnvironmentValue, Execution, JobContextManifest,
    JobEdge, PlacementCandidate, PlacementKind, ResolvedJob, ResolvedRun, ResolvedRunOptions,
    ResolvedTaskUnit, RetryPolicy, RunSubmission, Session, WorkspaceDescriptor,
};
use tak_loader::V2AuthoredRoot;

use super::RunCliArgs;

mod graph;
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
        let (session, affinity) = effective_session_and_affinity(&root.module, task)?;
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
            placement_candidates: placement_candidates(&root.module, task)?,
            retry: RetryPolicy::default(),
            idempotent: task.idempotent,
            queue: None,
            limiter_claims: Vec::new(),
            affinity,
            session,
            context_manifest: JobContextManifest {
                paths: context_paths.clone(),
            },
            pass_env_names,
        });
    }
    let job_edges = tasks
        .iter()
        .flat_map(|task| {
            task.dependencies.iter().map(|dependency| JobEdge {
                dependency_job_id: job_ids[dependency].clone(),
                dependent_job_id: task.job_id.clone(),
            })
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
        jobs,
        job_edges,
        limiter_definitions: Vec::new(),
        queue_definitions: Vec::new(),
    };
    let environment_values = environment_values(&run)?;
    Ok(RunSubmission::new(
        format!("submit-{}", uuid::Uuid::new_v4()),
        run,
        environment_values,
    )?)
}

fn effective_env_names(
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

fn environment_values(run: &ResolvedRun) -> Result<Vec<EnvironmentValue>> {
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

fn effective_session_and_affinity(
    module: &AuthoredModule,
    task: &AuthoredTask,
) -> Result<(Option<Session>, Option<Affinity>)> {
    let execution = task
        .execution
        .as_ref()
        .or(module.defaults.execution.as_ref());
    let session = execution.and_then(|execution| match execution {
        Execution::LocalOnly { local } => local.session.as_deref(),
        Execution::RemoteOnly { remote } => remote.session.as_deref(),
    });
    let affinity = match session {
        Some(session) => session.effective_affinity(task.affinity.as_ref())?,
        None => task.affinity.clone(),
    };
    Ok((session.cloned(), affinity))
}

fn placement_candidates(
    module: &AuthoredModule,
    task: &AuthoredTask,
) -> Result<Vec<PlacementCandidate>> {
    match task
        .execution
        .as_ref()
        .or(module.defaults.execution.as_ref())
    {
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
