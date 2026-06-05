use std::sync::Arc;

use prost::Message;
use tak_core::model::TaskLabel;
use tak_proto::{AppendWorkspaceUploadResponse, WorkspaceUploadRef};

use super::failures::{submit_decode_error, submit_protocol_error, submit_transport_error};
use super::{WorkspaceUploadOutcome, stream_upload_timeout};
use crate::engine::TaskOutputObserver;
use crate::engine::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget};
use crate::engine::remote_submit_failure::RemoteSubmitFailure;

pub(super) async fn stream_upload_for_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace: &RemoteWorkspaceStage,
    task_label: Option<&TaskLabel>,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
) -> Result<WorkspaceUploadOutcome, RemoteSubmitFailure> {
    let upload_id = upload_id(task_run_id, attempt, &workspace.sha256);
    let streamed = crate::engine::protocol_result_http::stream_workspace_upload_via_daemon(
        crate::engine::protocol_result_http::DaemonWorkspaceUploadStreamRequest {
            target,
            upload_id: &upload_id,
            archive_path: &workspace.archive_path,
            offset: 0,
            size_bytes: workspace.archive_byte_len,
            sha256: &workspace.sha256,
            timeout: stream_upload_timeout(workspace.archive_byte_len),
            progress: task_label.map(|task_label| {
                crate::engine::protocol_result_http::StreamUploadProgress {
                    observer: output_observer,
                    task_label,
                    attempt,
                }
            }),
        },
    )
    .await
    .map_err(submit_transport_error)?;
    if streamed.response.status != 200 {
        return Err(submit_protocol_error(
            target,
            "workspace upload stream",
            streamed.response.status,
        ));
    }
    let parsed = AppendWorkspaceUploadResponse::decode(streamed.response.body.as_slice())
        .map_err(|_| submit_decode_error(target, "workspace upload stream"))?;
    if !parsed.complete {
        return Err(RemoteSubmitFailure::other(format!(
            "infra error: remote node {} workspace upload stream stopped at byte {}",
            streamed.peer_node_id, parsed.offset
        )));
    }
    Ok(WorkspaceUploadOutcome {
        upload: Some(WorkspaceUploadRef {
            upload_id,
            sha256: workspace.sha256.clone(),
            size_bytes: workspace.archive_byte_len,
        }),
        preferred_node_id: Some(streamed.peer_node_id),
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
