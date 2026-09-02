use sha2::{Digest, Sha256};
use tak_proto::worker_v2::WorkerTerminalOutcome;

use crate::support::{
    v2_worker_paths,
    v2_worker_shared::{send, wait_terminal},
    worker_http::start_server,
};

#[tokio::test]
async fn worker_restores_a_paths_snapshot_into_each_private_job() {
    let server = start_server().await;
    let warm = v2_worker_paths::warm();
    send(&server, &warm, &[]).await;
    assert_eq!(
        wait_terminal(&server, &warm)
            .await
            .terminal
            .unwrap()
            .outcome,
        WorkerTerminalOutcome::Succeeded
    );
    let identity = serde_json::to_vec(&("run-1", "builder-a", "compiler")).unwrap();
    let cache_key = format!("path-cache:{:x}", Sha256::digest(identity));
    assert!(
        crate::support::v2_worker_capacity::snapshot(&server)
            .await
            .cached_content
            .contains(&cache_key)
    );

    let consume = v2_worker_paths::consume();
    send(&server, &consume, &[]).await;
    let terminal = wait_terminal(&server, &consume).await.terminal.unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Succeeded);
    assert!(terminal.outputs.is_empty());
}
