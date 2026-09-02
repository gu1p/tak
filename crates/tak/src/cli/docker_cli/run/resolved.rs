use std::num::NonZeroU32;

use anyhow::Result;
use tak_core::v2::{
    JobContextManifest, ResolvedJob, ResolvedRun, ResolvedRunOptions, ResolvedTaskUnit,
    ResourceRequest, RetryPolicy, RunSubmission, Step, WorkspaceDescriptor,
};

use super::super::{DockerCliSelectors, run_spec::DockerRunSpec};

mod environment;
mod placement;
mod runtime;

pub(super) async fn submission(
    selectors: &DockerCliSelectors,
    spec: &DockerRunSpec,
    workspace: WorkspaceDescriptor,
) -> Result<RunSubmission> {
    let (pass_env_names, environment_values) = environment::passed(&spec.pass_env)?;
    let runtime = runtime::from_spec(spec)?;
    let (placement_policy, placement_candidates) = placement::resolve(selectors).await?;
    let context_paths = workspace
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let task = ResolvedTaskUnit {
        task_id: "//:docker-run".into(),
        job_id: "job-0".into(),
        dependencies: Vec::new(),
        steps: vec![Step::Cmd {
            argv: spec.argv.clone(),
            cwd: spec.workdir.clone(),
            env: environment::step(&spec.env)?,
        }],
        outputs: Vec::new(),
        pass_env_names: pass_env_names.clone(),
        idempotent: false,
        affinity: None,
        timeout_s: None,
        runtime: Some(runtime),
    };
    let job = ResolvedJob {
        job_id: "job-0".into(),
        task_ids: vec![task.task_id.clone()],
        placement_policy,
        placement_candidates,
        resources: ResourceRequest::default(),
        retry: RetryPolicy::default(),
        idempotent: false,
        queue: None,
        queue_slots: NonZeroU32::MIN,
        queue_priority: 0,
        limiter_claims: Vec::new(),
        affinity: None,
        session: None,
        context_manifest: JobContextManifest {
            paths: context_paths,
        },
        pass_env_names,
    };
    RunSubmission::new(
        format!("submit-{}", uuid::Uuid::new_v4()),
        ResolvedRun {
            project_id: "tak-docker-run".into(),
            targets: vec![task.task_id.clone()],
            options: ResolvedRunOptions {
                max_parallel_jobs: NonZeroU32::MIN,
                keep_going: false,
            },
            workspace,
            tasks: vec![task],
            jobs: vec![job],
            job_edges: Vec::new(),
            limiter_definitions: Vec::new(),
            queue_definitions: Vec::new(),
        },
        environment_values,
    )
    .map_err(Into::into)
}
