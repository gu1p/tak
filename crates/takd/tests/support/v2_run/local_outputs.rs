use std::collections::BTreeMap;

use tak_core::v2::{JobEdge, OutputSelector, RunSubmission, Session, SessionReuse, Step};

use super::submission;

pub fn dependent_run(key: &str) -> RunSubmission {
    let mut request = submission(key, "secret");
    let session = Session::new("build", SessionReuse::Workspace, None).unwrap();
    request.run.tasks[0].task_id = "//:produce".into();
    request.run.tasks[0].steps = vec![shell(
        "mkdir -p generated scratch; printf producer > generated/input.txt; printf leak > scratch/leak.txt",
    )];
    request.run.tasks[0].outputs = vec![path("generated/input.txt")];
    request.run.jobs[0].task_ids = vec!["//:produce".into()];
    request.run.jobs[0].session = Some(session.clone());
    let mut consumer = request.run.tasks[0].clone();
    consumer.task_id = "//:consume".into();
    consumer.job_id = "job-1".into();
    consumer.dependencies = vec!["//:produce".into()];
    consumer.steps = vec![shell(
        "test \"$(cat generated/input.txt)\" = producer && test ! -e scratch/leak.txt && mkdir -p dist && printf producer-consumed > dist/result.txt",
    )];
    consumer.outputs = vec![path("dist/result.txt")];
    let mut consumer_job = request.run.jobs[0].clone();
    consumer_job.job_id = "job-1".into();
    consumer_job.task_ids = vec!["//:consume".into()];
    request.run.tasks.push(consumer);
    request.run.jobs.push(consumer_job);
    request.run.targets = vec!["//:consume".into()];
    request.run.job_edges = vec![JobEdge {
        dependency_job_id: "job-0".into(),
        dependent_job_id: "job-1".into(),
    }];
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

pub fn failed_keep_going_run(key: &str, producer_first: bool) -> RunSubmission {
    failed_run(key, producer_first, true)
}

pub fn failed_run(key: &str, producer_first: bool, keep_going: bool) -> RunSubmission {
    let mut request = submission(key, "secret");
    request.run.options.keep_going = keep_going;
    request.run.options.max_parallel_jobs = std::num::NonZeroU32::new(2).unwrap();
    let mut producer = request.run.tasks[0].clone();
    producer.task_id = "//:produce".into();
    producer.steps = vec![shell("mkdir -p dist && printf kept > dist/survivor.txt")];
    producer.outputs = vec![path("dist/survivor.txt")];
    let mut failure = producer.clone();
    failure.task_id = "//:fail".into();
    failure.job_id = "job-1".into();
    failure.steps = vec![shell("exit 7")];
    failure.outputs.clear();
    let mut producer_job = request.run.jobs[0].clone();
    producer_job.task_ids = vec![producer.task_id.clone()];
    let mut failure_job = producer_job.clone();
    failure_job.job_id = failure.job_id.clone();
    failure_job.task_ids = vec![failure.task_id.clone()];
    if producer_first {
        request.run.tasks = vec![producer, failure];
        request.run.jobs = vec![producer_job, failure_job];
    } else {
        request.run.tasks = vec![failure, producer];
        request.run.jobs = vec![failure_job, producer_job];
    }
    request.run.targets = vec!["//:produce".into(), "//:fail".into()];
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

fn shell(script: &str) -> Step {
    Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}

fn path(value: &str) -> OutputSelector {
    OutputSelector::Path {
        value: value.into(),
    }
}
