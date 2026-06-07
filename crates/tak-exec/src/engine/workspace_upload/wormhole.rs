use prost::Message;
use tak_proto::{StartWorkspaceWormholeUploadResponse, WorkspaceUploadRef};

use super::failures::{submit_decode_error, submit_protocol_error, submit_transport_error};
use super::{WorkspaceUploadOutcome, stream_upload_timeout};
use crate::engine::protocol_result_http::{
    DaemonWorkspaceWormholeUploadRequest, send_workspace_wormhole_via_daemon,
};
use crate::engine::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget};
use crate::engine::remote_submit_failure::RemoteSubmitFailure;

pub(super) async fn wormhole_upload_for_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace: &RemoteWorkspaceStage,
) -> Result<WorkspaceUploadOutcome, RemoteSubmitFailure> {
    let upload_id = upload_id(task_run_id, attempt, &workspace.sha256);
    let transferred = send_workspace_wormhole_via_daemon(DaemonWorkspaceWormholeUploadRequest {
        target,
        upload_id: &upload_id,
        archive_path: &workspace.archive_path,
        size_bytes: workspace.archive_byte_len,
        sha256: &workspace.sha256,
        timeout: stream_upload_timeout(workspace.archive_byte_len),
    })
    .await
    .map_err(submit_transport_error)?;
    if transferred.response.status != 200 {
        return Err(submit_protocol_error(
            target,
            "workspace wormhole upload",
            transferred.response.status,
        ));
    }
    let parsed = StartWorkspaceWormholeUploadResponse::decode(transferred.response.body.as_slice())
        .map_err(|_| submit_decode_error(target, "workspace wormhole upload"))?;
    if !parsed.complete {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} workspace wormhole upload did not complete",
            transferred.peer_node_id
        )));
    }
    Ok(WorkspaceUploadOutcome {
        upload: Some(WorkspaceUploadRef {
            upload_id,
            sha256: workspace.sha256.clone(),
            size_bytes: workspace.archive_byte_len,
        }),
        preferred_node_id: Some(transferred.peer_node_id),
    })
}

fn upload_id(task_run_id: &str, _attempt: u32, sha256: &str) -> String {
    format!("{task_run_id}-{sha256}")
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_') {
                value
            } else {
                '_'
            }
        })
        .collect()
}
