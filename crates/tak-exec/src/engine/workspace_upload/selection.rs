use crate::engine::remote_models::StrictRemoteTarget;
use crate::engine::remote_submit_failure::RemoteSubmitFailure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceTransferChoice {
    DirectChunks,
    TorStream,
    WormholeWithTorFallback,
    WormholeRequired,
}

pub(crate) fn selected_workspace_transfer_for_target(
    target: &StrictRemoteTarget,
) -> Result<WorkspaceTransferChoice, RemoteSubmitFailure> {
    if !super::super::transport::uses_tor_broker(target).unwrap_or(false) {
        return Ok(WorkspaceTransferChoice::DirectChunks);
    }
    match std::env::var("TAK_REMOTE_WORKSPACE_TRANSFER")
        .unwrap_or_default()
        .trim()
    {
        "" | "wormhole" => Ok(WorkspaceTransferChoice::WormholeWithTorFallback),
        "tor-stream" => Ok(WorkspaceTransferChoice::TorStream),
        "wormhole-required" => Ok(WorkspaceTransferChoice::WormholeRequired),
        value => Err(RemoteSubmitFailure::other(format!(
            "unsupported TAK_REMOTE_WORKSPACE_TRANSFER `{value}`; allowed values: tor-stream, wormhole, wormhole-required"
        ))),
    }
}
