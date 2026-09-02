use std::collections::BTreeMap;

use tak_core::v2::{OutputSelector, Step};
use tak_proto::worker_v2::{DispatchAttemptRequest, WorkerWorkspaceReuse, payload_digest};

use super::v2_worker::dispatch;

pub fn warm() -> DispatchAttemptRequest {
    request(
        "job-warm",
        "fence-warm",
        "mkdir -p .cache; printf warm > .cache/value",
    )
}

pub fn consume() -> DispatchAttemptRequest {
    request(
        "job-consume",
        "fence-consume",
        "test \"$(cat .cache/value)\" = warm",
    )
}

fn request(job_id: &str, fence: &str, script: &str) -> DispatchAttemptRequest {
    let mut request = dispatch(1, 1, fence);
    request.identity.job_id = job_id.into();
    request.payload.tasks[0].job_id = job_id.into();
    request.payload.tasks[0].task_id = format!("//:{job_id}");
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload.workspace_reuse = WorkerWorkspaceReuse::Paths {
        session_id: "compiler".into(),
        paths: vec![OutputSelector::Path {
            value: ".cache".into(),
        }],
    };
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}
