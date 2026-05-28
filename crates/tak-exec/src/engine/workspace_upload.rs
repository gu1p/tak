use std::time::Duration;

use base64::Engine;
use prost::Message;
use sha2::{Digest, Sha256};
use tak_proto::{
    AppendWorkspaceUploadResponse, BeginWorkspaceUploadRequest, BeginWorkspaceUploadResponse,
    FinishWorkspaceUploadResponse, WorkspaceUploadRef,
};

use super::protocol_result_http::remote_protocol_http_request;
use super::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget};
use super::remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind};

mod failures;
mod requests;

use failures::{submit_decode_error, submit_protocol_error, submit_transport_error};
use requests::{append_chunk_request, begin_upload_request, finish_upload_request};

const WORKSPACE_UPLOAD_CHUNK_BYTES: usize = 1024 * 1024;

pub(crate) async fn upload_workspace_for_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    workspace: &RemoteWorkspaceStage,
) -> Result<Option<WorkspaceUploadRef>, RemoteSubmitFailure> {
    if super::transport::uses_tor_broker(target).unwrap_or(false) {
        return Ok(None);
    }
    let archive = decode_workspace_archive(workspace)?;
    let sha256 = format!("{:x}", Sha256::digest(&archive));
    let size_bytes = archive.len() as u64;
    let mut begin = begin_upload(target, task_run_id, attempt, &sha256, size_bytes).await?;
    let Some(begin) = begin.take() else {
        return Ok(None);
    };
    upload_and_finish_chunks(target, &begin.upload_id, &archive, begin.offset).await?;
    Ok(Some(WorkspaceUploadRef {
        upload_id: begin.upload_id,
        sha256,
        size_bytes,
    }))
}

fn decode_workspace_archive(
    workspace: &RemoteWorkspaceStage,
) -> Result<Vec<u8>, RemoteSubmitFailure> {
    base64::engine::general_purpose::STANDARD
        .decode(&workspace.archive_zip_base64)
        .map_err(|err| RemoteSubmitFailure {
            kind: RemoteSubmitFailureKind::Other,
            message: format!("failed decoding staged workspace archive: {err}"),
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

async fn upload_chunks(
    target: &StrictRemoteTarget,
    upload_id: &str,
    archive: &[u8],
    offset: u64,
) -> Result<(), RemoteSubmitFailure> {
    let mut offset = offset as usize;
    while offset < archive.len() {
        let end = archive.len().min(offset + WORKSPACE_UPLOAD_CHUNK_BYTES);
        offset = append_chunk(target, upload_id, offset as u64, &archive[offset..end]).await?;
    }
    Ok(())
}

async fn upload_and_finish_chunks(
    target: &StrictRemoteTarget,
    upload_id: &str,
    archive: &[u8],
    offset: u64,
) -> Result<(), RemoteSubmitFailure> {
    let mut offset = offset as usize;
    for _ in 0..2 {
        upload_chunks(target, upload_id, archive, offset as u64).await?;
        match finish_upload(target, upload_id).await? {
            FinishUpload::Complete => return Ok(()),
            FinishUpload::Incomplete { next_offset } => offset = next_offset,
        }
    }
    Err(RemoteSubmitFailure {
        kind: RemoteSubmitFailureKind::Other,
        message: format!(
            "infra error: remote node {} workspace upload finish did not complete",
            target.node_id
        ),
    })
}

async fn append_chunk(
    target: &StrictRemoteTarget,
    upload_id: &str,
    offset: u64,
    chunk: &[u8],
) -> Result<usize, RemoteSubmitFailure> {
    let path = format!("/v2/workspaces/uploads/{upload_id}?offset={offset}");
    let (status, response) = append_chunk_request(target, &path, chunk).await?;
    if status != 200 && status != 409 {
        return Err(submit_protocol_error(
            target,
            "workspace upload chunk",
            status,
        ));
    }
    let parsed = AppendWorkspaceUploadResponse::decode(response.as_slice())
        .map_err(|_| submit_decode_error(target, "workspace upload chunk"))?;
    Ok(parsed.offset as usize)
}

async fn finish_upload(
    target: &StrictRemoteTarget,
    upload_id: &str,
) -> Result<FinishUpload, RemoteSubmitFailure> {
    let path = format!("/v2/workspaces/uploads/{upload_id}/finish");
    let (status, response) = finish_upload_request(target, &path).await?;
    if status == 409 {
        let parsed = AppendWorkspaceUploadResponse::decode(response.as_slice())
            .map_err(|_| submit_decode_error(target, "workspace upload finish"))?;
        return Ok(FinishUpload::Incomplete {
            next_offset: parsed.offset as usize,
        });
    }
    if status != 200 {
        return Err(submit_protocol_error(
            target,
            "workspace upload finish",
            status,
        ));
    }
    FinishWorkspaceUploadResponse::decode(response.as_slice())
        .map(|_| FinishUpload::Complete)
        .map_err(|_| submit_decode_error(target, "workspace upload finish"))
}

enum FinishUpload {
    Complete,
    Incomplete { next_offset: usize },
}

fn upload_timeout() -> Duration {
    Duration::from_secs(30)
}
