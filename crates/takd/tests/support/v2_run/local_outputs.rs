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
