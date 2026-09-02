use tak_proto::worker_v2::WorkerTerminalOutcome;

use crate::support::{
    v2_worker_shared::{dispatch_with_seed, seed_archive, send, wait_terminal},
    worker_http::start_server,
};

#[path = "v2_worker_shared_workspace_base_behavior/context.rs"]
mod context;

#[tokio::test]
async fn remote_shared_workspace_rejects_reuse_with_a_different_base() {
    let server = start_server().await;
    let first = dispatch_with_seed(1, "fence-1", "alpha", true);
    send(&server, &first, &seed_archive("alpha")).await;
    assert_eq!(
        wait_terminal(&server, &first)
            .await
            .terminal
            .unwrap()
            .outcome,
        WorkerTerminalOutcome::Succeeded
    );

    let mismatched = dispatch_with_seed(2, "fence-2", "beta", true);
    send(&server, &mismatched, &seed_archive("beta")).await;
    let observed = wait_terminal(&server, &mismatched).await;
    assert_eq!(
        observed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Failed
    );
    assert!(
        observed.events.is_empty(),
        "mismatched base executed a task"
    );
}
