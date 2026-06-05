use std::time::Duration;

use prost::Message;
use std::sync::Arc;
use tak_core::model::TaskLabel;
use tak_proto::{BeginWorkspaceUploadRequest, BeginWorkspaceUploadResponse, WorkspaceUploadRef};

use super::TaskOutputObserver;
use super::protocol_result_http::remote_protocol_http_request;
use super::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget};
use super::remote_submit_failure::RemoteSubmitFailure;

mod failures;
mod legacy;
mod requests;
mod stream;

use failures::{submit_decode_error, submit_protocol_error, submit_transport_error};
use legacy::upload_and_finish_chunks;
use requests::begin_upload_request;
use stream::stream_upload_for_submit;

pub(crate) async fn upload_workspace_for_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace: &RemoteWorkspaceStage,
    task_label: Option<&TaskLabel>,
    output_observer: Option<&Arc<dyn TaskOutputObserver>>,
) -> Result<WorkspaceUploadOutcome, RemoteSubmitFailure> {
    if super::transport::uses_tor_broker(target).unwrap_or(false) {
        return stream_upload_for_submit(
            target,
            task_run_id,
            attempt,
            workspace,
            task_label,
            output_observer,
        )
        .await;
    }
    let archive = read_workspace_archive(workspace)?;
    let sha256 = workspace.sha256.clone();
    let size_bytes = workspace.archive_byte_len;
    let mut begin = begin_upload(target, task_run_id, attempt, &sha256, size_bytes).await?;
    let Some(begin) = begin.take() else {
        return Ok(WorkspaceUploadOutcome::inline());
    };
    upload_and_finish_chunks(target, &begin.upload_id, &archive, begin.offset).await?;
    Ok(WorkspaceUploadOutcome {
        upload: Some(WorkspaceUploadRef {
            upload_id: begin.upload_id,
            sha256,
            size_bytes,
        }),
        preferred_node_id: None,
    })
}

#[derive(Debug)]
pub(crate) struct WorkspaceUploadOutcome {
    pub(crate) upload: Option<WorkspaceUploadRef>,
    pub(crate) preferred_node_id: Option<String>,
}

impl WorkspaceUploadOutcome {
    fn inline() -> Self {
        Self {
            upload: None,
            preferred_node_id: None,
        }
    }
}

fn read_workspace_archive(
    workspace: &RemoteWorkspaceStage,
) -> Result<Vec<u8>, RemoteSubmitFailure> {
    std::fs::read(&workspace.archive_path).map_err(|err| {
        RemoteSubmitFailure::other(format!(
            "failed reading staged workspace archive {}: {err}",
            workspace.archive_path.display()
        ))
    })
}

async fn begin_upload(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    sha256: &str,
    size_bytes: u64,
) -> Result<Option<BeginWorkspaceUploadResponse>, RemoteSubmitFailure> {
    let body = BeginWorkspaceUploadRequest {
        task_run_id: task_run_id.to_string(),
        attempt,
        sha256: sha256.to_string(),
        size_bytes,
    }
    .encode_to_vec();
    let (status, response) = begin_upload_request(target, &body).await?;
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        return Err(submit_protocol_error(
            target,
            "workspace upload begin",
            status,
        ));
    }
    BeginWorkspaceUploadResponse::decode(response.as_slice())
        .map(Some)
        .map_err(|_| submit_decode_error(target, "workspace upload begin"))
}

fn upload_timeout() -> Duration {
    Duration::from_secs(30)
}

pub(super) fn stream_upload_timeout(size_bytes: u64) -> Duration {
    Duration::from_secs(120 + size_bytes.div_ceil(24 * 1024))
}
