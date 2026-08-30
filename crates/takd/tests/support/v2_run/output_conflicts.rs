use tak_core::v2::{JobEdge, ResolvedJob, ResolvedTaskUnit, RunSubmission};

use super::submission;
use builders::{consumer, edge, producer, scheduled};

mod builders;

pub fn dependency(key: &str) -> RunSubmission {
    let request = submission(key, "secret");
    let task = request.run.tasks[0].clone();
    let job = request.run.jobs[0].clone();
    finish(
        request,
        vec![
            producer(&task, "left", "job-left", "left"),
            producer(&task, "right", "job-right", "right"),
            consumer(&task),
        ],
        vec![
            scheduled(&job, "job-left", "//:left"),
            scheduled(&job, "job-right", "//:right"),
            scheduled(&job, "job-consume", "//:consume"),
        ],
        vec!["//:consume".into()],
        vec![
            edge("job-left", "job-consume"),
            edge("job-right", "job-consume"),
        ],
    )
}

pub fn final_sink(key: &str) -> RunSubmission {
    let request = submission(key, "secret");
    let task = request.run.tasks[0].clone();
    let job = request.run.jobs[0].clone();
    finish(
        request,
        vec![
            producer(&task, "left", "job-left", "left"),
            producer(&task, "right", "job-right", "right"),
        ],
        vec![
            scheduled(&job, "job-left", "//:left"),
            scheduled(&job, "job-right", "//:right"),
        ],
        vec!["//:left".into(), "//:right".into()],
        vec![],
    )
}

fn finish(
    mut request: RunSubmission,
    tasks: Vec<ResolvedTaskUnit>,
    jobs: Vec<ResolvedJob>,
    targets: Vec<String>,
    edges: Vec<JobEdge>,
) -> RunSubmission {
    request.run.tasks = tasks;
    request.run.jobs = jobs;
    request.run.targets = targets;
    request.run.job_edges = edges;
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}
