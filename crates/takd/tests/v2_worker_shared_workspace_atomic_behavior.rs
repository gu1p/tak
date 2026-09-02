use tak_proto::worker_v2::WorkerTerminalOutcome;

use crate::support::{
    worker_http::start_server,
    v2_worker_shared::{dispatch_with_seed, seed_archive, send, wait_terminal},
};

#[tokio::test]
async fn failed_remote_shared_initialization_does_not_publish_a_partial_workspace() {
    let server = start_server().await;
    let invalid = dispatch_with_seed(1, "fence-1", "invalid", false);
    send(&server, &invalid, &seed_archive("invalid")).await;
    let failed = wait_terminal(&server, &invalid).await;
    assert_eq!(
        failed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Failed
    );

    let valid = dispatch_with_seed(2, "fence-2", "valid", true);
    send(&server, &valid, &seed_archive("valid")).await;
    let completed = wait_terminal(&server, &valid).await;
    assert_eq!(
        completed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Succeeded
    );
}
