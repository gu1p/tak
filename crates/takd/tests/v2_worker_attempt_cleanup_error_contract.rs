use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};
use tak_proto::worker_v2::{WorkerTerminalOutcome, payload_digest};

use crate::support::{
    worker_http::start_server,
    v2_worker_cleanup::{assert_cleanup, private_attempt_root, seed_preserved_roots},
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_shared::{send, wait_terminal},
};

#[tokio::test]
async fn setup_error_removes_only_its_private_root_after_failed_terminal_persistence() {
    let server = start_server().await;
    let mut request = output_dispatch();
    request.identity.run_id = "run-error".into();
    request.identity.job_id = "job-error".into();
    request.identity.fencing_token = "fence-error".into();
    request.payload.tasks[0].job_id = "job-error".into();
    request.payload.workspace.descriptor.manifest =
        WorkspaceManifest::new([WorkspaceEntry::directory("missing").unwrap()]).unwrap();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    let attempt_root = private_attempt_root(&server, &request);
    let preserved = seed_preserved_roots(&server);

    send(&server, &request, &output_archive()).await;
    let observed = wait_terminal(&server, &request).await;

    assert_eq!(
        observed.terminal.unwrap().outcome,
        WorkerTerminalOutcome::Failed
    );
    assert_cleanup(&attempt_root, &preserved).await;
}
