#![cfg(test)]

use super::workspace_upload::upload_workspace_for_submit;
use super::workspace_upload_tor_stream_test_support::{
    EnvVarGuard, TorStreamUploadDaemon, tor_target, workspace_stage,
};

async fn upload_prepared_workspace(
    workspace: &super::remote_models::RemoteWorkspaceStage,
) -> Result<
    super::workspace_upload::WorkspaceUploadOutcome,
    super::remote_submit_failure::RemoteSubmitFailure,
> {
    upload_workspace_for_submit(&tor_target(), "run-1", 1, workspace, None, None).await
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_falls_back_to_tor_stream_when_remote_route_is_unsupported() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");
    let archive = b"fallback archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_unsupported_wormhole(&archive).await;
    let workspace = workspace_stage(&archive);

    let outcome = upload_prepared_workspace(&workspace)
        .await
        .expect("fallback upload");

    assert_eq!(
        outcome.preferred_node_id.as_deref(),
        Some("builder-selected")
    );
    assert_eq!(daemon.wormhole_attempts().await, 1);
    assert_eq!(daemon.bytes().await, archive);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_retries_retryable_failures_before_tor_stream_fallback() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");
    let archive = b"retryable wormhole archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_retryable_wormhole_error(&archive).await;
    let workspace = workspace_stage(&archive);

    let outcome = upload_prepared_workspace(&workspace)
        .await
        .expect("fallback upload");

    assert_eq!(
        outcome.preferred_node_id.as_deref(),
        Some("builder-selected")
    );
    assert_eq!(daemon.wormhole_attempts().await, 4);
    assert_eq!(daemon.bytes().await, archive);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_required_does_not_fall_back_to_tor_stream() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole-required");
    let archive = b"strict archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_unsupported_wormhole(&archive).await;
    let workspace = workspace_stage(&archive);

    let err = upload_prepared_workspace(&workspace)
        .await
        .expect_err("required wormhole failure");

    assert!(err.message.contains("workspace wormhole upload"));
    assert_eq!(daemon.wormhole_attempts().await, 1);
    assert!(daemon.bytes().await.is_empty());
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_required_retries_retryable_failures_without_tor_stream_fallback() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole-required");
    let archive = b"strict retryable archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_retryable_wormhole_error(&archive).await;
    let workspace = workspace_stage(&archive);

    let err = upload_prepared_workspace(&workspace)
        .await
        .expect_err("required wormhole failure");

    assert!(err.message.contains("temporary wormhole preflight failure"));
    assert_eq!(daemon.wormhole_attempts().await, 4);
    assert!(daemon.bytes().await.is_empty());
}
