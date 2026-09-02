use std::time::Duration;

use sha2::{Digest, Sha256};
use tak_core::v2::Step;
use tak_proto::worker_v2::{DispatchAttemptRequest, payload_digest};

use super::{
    v2_worker_http::{post, status},
    v2_worker_shared::{dispatch_with_seed, seed_archive, send, wait_terminal},
    worker_http::RunningServer,
};

pub fn request(run: &str, fence: &str, script: &str) -> DispatchAttemptRequest {
    let mut request = dispatch_with_seed(1, fence, run, true);
    request.identity.run_id = run.into();
    request.identity.job_id = format!("job-{run}");
    request.payload.tasks[0]
        .job_id
        .clone_from(&request.identity.job_id);
    let Step::Cmd { argv, .. } = &mut request.payload.tasks[0].steps[0] else {
        unreachable!();
    };
    argv[2] = script.into();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub async fn dispatch(server: &RunningServer, request: &DispatchAttemptRequest) {
    send(server, request, &seed_archive(&request.identity.run_id)).await;
}

pub async fn release_terminal_run(server: &RunningServer, request: &DispatchAttemptRequest) {
    let digest = wait_terminal(server, request)
        .await
        .terminal
        .unwrap()
        .terminal_digest;
    let body = serde_json::to_vec(&serde_json::json!({
        "protocol_version": 2,
        "identity": request.identity,
        "terminal_digest": digest,
        "run_terminal": true
    }))
    .unwrap();
    for _ in 0..2 {
        assert_eq!(
            status(&post(server, "/v2/attempts/ack", Some("secret"), &["v2"], &body).await),
            200
        );
    }
}

pub fn root(server: &RunningServer, request: &DispatchAttemptRequest) -> std::path::PathBuf {
    let key = serde_json::to_string(&(&request.identity.run_id, "session-a")).unwrap();
    server
        .state_root
        .join("worker-v2-shared")
        .join(format!("{:x}", Sha256::digest(key)))
}

pub async fn wait_for(path: &std::path::Path, exists: bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while path.exists() != exists {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
