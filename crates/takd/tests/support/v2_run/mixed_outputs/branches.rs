use tak_core::v2::RunSubmission;

use super::super::submission;
use super::builders::{authored_task, edge, finish};
use super::placement::{local_job, remote_job};

pub fn conflicting(key: &str) -> RunSubmission {
    independent(key, "right")
}

pub fn identical(key: &str) -> RunSubmission {
    independent(key, "same")
}

fn independent(key: &str, remote_value: &str) -> RunSubmission {
    let request = submission(key, "secret");
    let task = &request.run.tasks[0];
    finish(
        request.clone(),
        vec![
            authored_task(
                task,
                "//:left",
                "job-left",
                &[],
                "mkdir -p shared; printf same > shared/value.txt",
                "shared/value.txt",
            ),
            authored_task(
                task,
                "//:right",
                "job-right",
                &[],
                &format!("mkdir -p shared; printf {remote_value} > shared/value.txt"),
                "shared/value.txt",
            ),
            authored_task(
                task,
                "//:consumer",
                "job-consumer",
                &["//:left", "//:right"],
                "test \"$(cat shared/value.txt)\" = same; mkdir -p dist; printf consumed > dist/result.txt",
                "dist/result.txt",
            ),
        ],
        vec![
            local_job(&request.run.jobs[0], "job-left", "//:left"),
            remote_job(&request.run.jobs[0], "job-right", "//:right"),
            remote_job(&request.run.jobs[0], "job-consumer", "//:consumer"),
        ],
        vec![
            edge("job-left", "job-consumer"),
            edge("job-right", "job-consumer"),
        ],
        "//:consumer",
        2,
    )
}
