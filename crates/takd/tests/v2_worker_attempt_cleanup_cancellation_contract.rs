use tak_proto::worker_v2::WorkerTerminalOutcome;

use crate::support::{
    worker_http::start_server,
    v2_worker_capacity::cancel,
    v2_worker_cleanup::{
        assert_cleanup, command_request, private_attempt_root, seed_preserved_roots,
    },
    v2_worker_execution::output_archive,
    v2_worker_shared::{send, wait_terminal},
};

#[tokio::test]
async fn cancelled_attempt_removes_only_its_private_root_after_terminal_persistence() {
    let server = start_server().await;
    let request = command_request(
        "run-cancelled",
        "job-cancelled",
        "fence-cancelled",
        "sleep 30",
    );
    let attempt_root = private_attempt_root(&server, &request);
    let preserved = seed_preserved_roots(&server);

    send(&server, &request, &output_archive()).await;
    cancel(&server, &request).await;
    let observed = wait_terminal(&server, &request).await;

    assert_eq!(
        observed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Cancelled
    );
    assert_cleanup(&attempt_root, &preserved).await;
}
