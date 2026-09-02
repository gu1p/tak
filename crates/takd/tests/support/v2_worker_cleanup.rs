use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tak_core::v2::Step;
use tak_proto::worker_v2::{DispatchAttemptRequest, payload_digest};

use super::v2_worker_execution::output_dispatch;
use super::worker_http::RunningServer;

pub fn command_request(run: &str, job: &str, fence: &str, command: &str) -> DispatchAttemptRequest {
    let mut request = output_dispatch();
    request.identity.run_id = run.into();
    request.identity.job_id = job.into();
    request.identity.fencing_token = fence.into();
    request.payload.tasks[0].job_id = job.into();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), command.into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}

pub fn private_attempt_root(server: &RunningServer, request: &DispatchAttemptRequest) -> PathBuf {
    server
        .state_root
        .join("worker-v2-attempts")
        .join(hash(&request.identity.fencing_token))
}

pub fn seed_preserved_roots(server: &RunningServer) -> Vec<PathBuf> {
    let roots = vec![
        server
            .state_root
            .join("worker-v2-attempts")
            .join(hash("unrelated-fence")),
        server.state_root.join("worker-v2-path-caches/retained"),
        server.state_root.join("worker-v2-shared/retained"),
    ];
    for root in &roots {
        std::fs::create_dir_all(root.join("marker")).unwrap();
    }
    roots
}

pub async fn assert_cleanup(attempt_root: &std::path::Path, preserved: &[PathBuf]) {
    tokio::time::timeout(Duration::from_secs(1), async {
        while attempt_root.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("terminal attempt root was retained");
    assert!(
        preserved.iter().all(|root| root.join("marker").is_dir()),
        "cleanup removed unrelated worker state"
    );
}

fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
