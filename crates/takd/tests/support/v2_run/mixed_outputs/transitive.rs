use tak_core::v2::RunSubmission;

use super::super::submission;
use super::builders::{authored_task, edge, finish};
use super::placement::{local_job, remote_job};

pub fn transitive(key: &str) -> RunSubmission {
    let request = submission(key, "secret");
    let task = &request.run.tasks[0];
    finish(
        request.clone(),
        vec![
            authored_task(
                task,
                "//:ancestor",
                "job-ancestor",
                &[],
                "mkdir -p graph; printf ancestor > graph/ancestor.txt",
                "graph/ancestor.txt",
            ),
            authored_task(
                task,
                "//:middle",
                "job-middle",
                &["//:ancestor"],
                "test \"$(cat graph/ancestor.txt)\" = ancestor; printf middle > graph/middle.txt",
                "graph/middle.txt",
            ),
            authored_task(
                task,
                "//:consumer",
                "job-consumer",
                &["//:middle"],
                "test \"$(cat graph/ancestor.txt)\" = ancestor; test \"$(cat graph/middle.txt)\" = middle; mkdir -p dist; printf ancestor+middle > dist/result.txt",
                "dist/result.txt",
            ),
        ],
        vec![
            remote_job(&request.run.jobs[0], "job-ancestor", "//:ancestor"),
            local_job(&request.run.jobs[0], "job-middle", "//:middle"),
            remote_job(&request.run.jobs[0], "job-consumer", "//:consumer"),
        ],
        vec![
            edge("job-ancestor", "job-middle"),
            edge("job-middle", "job-consumer"),
        ],
        "//:consumer",
        1,
    )
}
