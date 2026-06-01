use prost::Message;
use tak_proto::{AppendWorkspaceUploadResponse, FinishWorkspaceUploadResponse};

use super::failures::{submit_decode_error, submit_protocol_error};
use super::requests::{append_chunk_request, finish_upload_request};
use crate::engine::remote_models::StrictRemoteTarget;
use crate::engine::remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind};

const WORKSPACE_UPLOAD_CHUNK_BYTES: usize = 1024 * 1024;

pub(super) async fn upload_and_finish_chunks(
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
