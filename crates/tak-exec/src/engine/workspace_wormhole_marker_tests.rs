#![cfg(test)]

use std::time::Duration;

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
async fn wormhole_preflight_without_marker_falls_back_before_transfer() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");
    let archive = b"generic stream status archive body".to_vec();
    let daemon = TorStreamUploadDaemon::spawn_with_dropped_commits(&archive, Vec::new()).await;
    let workspace = workspace_stage(&archive);

    let outcome = tokio::time::timeout(
        Duration::from_secs(2),
        upload_prepared_workspace(&workspace),
    )
    .await
    .expect("fallback should not wait for magic-wormhole")
    .expect("fallback upload");

    assert_eq!(
        outcome.preferred_node_id.as_deref(),
        Some("builder-selected")
    );
    assert_eq!(daemon.wormhole_attempts().await, 1);
    assert_eq!(daemon.bytes().await, archive);
}
