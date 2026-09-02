use std::collections::BTreeMap;
use std::num::NonZeroU32;

use tak_core::v2::{
    EnvironmentValue, JobContextManifest, PlacementCandidate, PlacementKind, PlacementPolicy,
    RemoteRequirements, RemoteSelection, ResolvedJob, ResolvedRun, ResolvedRunOptions,
    ResolvedTaskUnit, ResourceRequest, RetryPolicy, RunSubmission, Step, WorkspaceDescriptor,
};

use super::{ExecCliArgs, runtime, *};

pub(in crate::cli) async fn submission(
    args: &ExecCliArgs,
    workspace: WorkspaceDescriptor,
) -> Result<RunSubmission> {
    let (pass_env_names, environment_values) = passed_environment(&args.pass_env)?;
    let step_env = step_environment(&args.env)?;
    let runtime = runtime::selected(args)?;
    let (placement_policy, placement_candidates) = placement(args.remote).await?;
    let resources = runtime
        .as_ref()
        .and_then(tak_core::v2::TaskRuntime::resources)
        .map_or_else(ResourceRequest::default, |limits| ResourceRequest {
            cpu_millis: limits.cpu_millis,
            memory_bytes: limits.memory_bytes,
            execution_slots: NonZeroU32::MIN,
        });
    let context_paths = workspace
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let task = ResolvedTaskUnit {
        task_id: "//:exec".into(),
        job_id: "job-0".into(),
        dependencies: Vec::new(),
        steps: vec![Step::Cmd {
            argv: args.argv.clone(),
            cwd: args.cwd.clone(),
            env: step_env,
        }],
        outputs: Vec::new(),
        pass_env_names: pass_env_names.clone(),
        idempotent: false,
        affinity: None,
        timeout_s: None,
        runtime,
    };
    let job = ResolvedJob {
        job_id: "job-0".into(),
        task_ids: vec![task.task_id.clone()],
        placement_policy,
        placement_candidates,
        resources,
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
            project_id: "tak-exec".into(),
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

fn passed_environment(names: &[String]) -> Result<(Vec<String>, Vec<EnvironmentValue>)> {
    let names = tak_core::v2::PassEnv::new(names)?
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

fn step_environment(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut environment = BTreeMap::new();
    for entry in entries {
        let Some((name, value)) = entry.split_once('=') else {
            bail!("invalid --env value `{entry}`; expected KEY=VALUE");
        };
        if name.is_empty() {
            bail!("invalid --env value `{entry}`; key cannot be empty");
        }
        environment.insert(name.into(), value.into());
    }
    Ok(environment)
}

async fn placement(remote: bool) -> Result<(PlacementPolicy, Vec<PlacementCandidate>)> {
    if !remote {
        return Ok((
            PlacementPolicy {
                policy_id: "local".into(),
                selection: RemoteSelection::Sequential,
            },
            vec![PlacementCandidate {
                node_id: "local".into(),
                kind: PlacementKind::Local,
                transport: None,
                reason: "local execution".into(),
                tier: 0,
                requirements: None,
            }],
        ));
    }
    let candidates = super::super::daemon_run::remote_candidates(RemoteRequirements {
        pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport: None,
    })
    .await?;
    if candidates.is_empty() {
        bail!("no connected protocol-v2 worker matches tak exec");
    }
    Ok((
        PlacementPolicy {
            policy_id: "exec-remote-balanced".into(),
            selection: RemoteSelection::Balanced,
        },
        candidates,
    ))
}
