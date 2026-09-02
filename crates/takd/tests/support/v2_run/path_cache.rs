use std::collections::BTreeMap;

use tak_core::v2::{JobEdge, OutputSelector, RunSubmission, Session, SessionReuse, Step};

use super::submission;

pub fn dependent_run(key: &str) -> RunSubmission {
    let mut request = submission(key, "secret");
    let session = Session::new(
        "compiler",
        SessionReuse::Paths {
            paths: vec![path(".cache")],
        },
        None,
    )
    .unwrap();
    request.run.tasks[0].task_id = "//:warm".into();
    request.run.tasks[0].steps = vec![shell(
        "mkdir -p .cache; printf warm > .cache/value; printf hidden > .cache/private",
    )];
    request.run.tasks[0].outputs.clear();
    request.run.jobs[0].task_ids = vec!["//:warm".into()];
    request.run.jobs[0].session = Some(session.clone());
    let mut consume = request.run.tasks[0].clone();
    consume.task_id = "//:consume".into();
    consume.job_id = "job-1".into();
    consume.dependencies = vec!["//:warm".into()];
    consume.steps = vec![shell(
        "test \"$(cat .cache/value)\" = warm; mkdir -p dist; printf restored > dist/result",
    )];
    consume.outputs = vec![path("dist/result")];
    let mut job = request.run.jobs[0].clone();
    job.job_id = "job-1".into();
    job.task_ids = vec!["//:consume".into()];
    request.run.tasks.push(consume);
    request.run.jobs.push(job);
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
