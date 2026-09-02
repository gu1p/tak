use std::collections::BTreeMap;
use std::num::NonZeroU32;

use tak_core::v2::{JobEdge, OutputSelector, ResolvedJob, ResolvedTaskUnit, RunSubmission, Step};

pub(super) fn authored_task(
    template: &ResolvedTaskUnit,
    task_id: &str,
    job_id: &str,
    dependencies: &[&str],
    script: &str,
    output: &str,
) -> ResolvedTaskUnit {
    ResolvedTaskUnit {
        task_id: task_id.into(),
        job_id: job_id.into(),
        dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
        steps: vec![shell(script)],
        outputs: vec![OutputSelector::Path {
            value: output.into(),
        }],
        ..template.clone()
    }
}

pub(super) fn finish(
    mut request: RunSubmission,
    tasks: Vec<ResolvedTaskUnit>,
    jobs: Vec<ResolvedJob>,
    edges: Vec<JobEdge>,
    target: &str,
    max_parallel: u32,
) -> RunSubmission {
    request.run.tasks = tasks;
    request.run.jobs = jobs;
    request.run.job_edges = edges;
    request.run.targets = vec![target.into()];
    request.run.options.max_parallel_jobs = NonZeroU32::new(max_parallel).unwrap();
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

pub(super) fn edge(dependency: &str, dependent: &str) -> JobEdge {
    JobEdge {
        dependency_job_id: dependency.into(),
        dependent_job_id: dependent.into(),
    }
}

fn shell(script: &str) -> Step {
    Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}
