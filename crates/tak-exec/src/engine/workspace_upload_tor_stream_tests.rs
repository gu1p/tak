#![cfg(test)]

use super::workspace_upload::upload_workspace_for_submit;
use super::workspace_upload_tor_stream_test_support::{
    EnvVarGuard, TorStreamUploadDaemon, tor_target, workspace_stage,
};

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn tor_stream_upload_resumes_from_worker_status_after_dropped_response() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "tor-stream");
    let archive = b"tor stream archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_dropped_commits(&archive, vec![8]).await;
    let workspace = workspace_stage(&archive);

    let outcome = upload_workspace_for_submit(&tor_target(), "run-1", 1, &workspace, None, None)
        .await
        .expect("upload");

    assert_eq!(
        outcome.preferred_node_id.as_deref(),
        Some("builder-selected")
    );
    assert_eq!(daemon.bytes().await, archive);
    assert_eq!(daemon.stream_offsets().await, vec![0, 8]);
    assert_eq!(daemon.status_nodes().await, vec!["builder-selected"]);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn tor_stream_upload_exhausts_retries_when_status_does_not_advance() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "tor-stream");
    let archive = b"never advances".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_without_progress(&archive).await;
    let workspace = workspace_stage(&archive);

    let err = upload_workspace_for_submit(&tor_target(), "run-1", 1, &workspace, None, None)
        .await
        .expect_err("retry exhaustion");

    assert!(
        err.message
            .contains("workspace upload stream retries exhausted")
    );
    assert!(err.message.contains("offset 0"));
    assert_eq!(daemon.stream_offsets().await, vec![0, 0, 0, 0]);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn tor_stream_upload_resets_retry_budget_after_committed_progress() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "tor-stream");
    let archive = b"progress keeps retrying past three failures".to_vec();
    let daemon =
        TorStreamUploadDaemon::spawn_with_dropped_commits(&archive, vec![8, 8, 8, 8]).await;
    let workspace = workspace_stage(&archive);

    upload_workspace_for_submit(&tor_target(), "run-1", 1, &workspace, None, None)
        .await
        .expect("upload");

    assert_eq!(daemon.bytes().await, archive);
    assert_eq!(daemon.stream_offsets().await, vec![0, 8, 16, 24, 32]);
}
