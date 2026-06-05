use std::path::Path;

use anyhow::{Context, Result, anyhow};
use prost::Message;
use tak_proto::AppendWorkspaceUploadResponse;

use super::super::errors::DaemonLocalError;
use super::super::types::DaemonPeerSnapshot;
use super::progress::ActiveStreamUploadProgress;
use super::status::{completed_status_response, upload_status};
use super::{DaemonWorkspaceUploadStreamRequest, send_stream_upload_request};
use crate::engine::StrictRemoteTarget;
use crate::engine::protocol_result_http::request::RemoteHttpResponse;

const MAX_RETRIES_PER_COMMITTED_OFFSET: u8 = 3;

pub(super) struct StreamUploadPlan<'a> {
    target: &'a StrictRemoteTarget,
    upload_id: &'a str,
    archive_path: &'a Path,
    offset: u64,
    size_bytes: u64,
    sha256: &'a str,
}

impl<'a> StreamUploadPlan<'a> {
    pub(super) fn from_request(request: &DaemonWorkspaceUploadStreamRequest<'a>) -> Self {
        Self {
            target: request.target,
            upload_id: request.upload_id,
            archive_path: request.archive_path,
            offset: request.offset,
            size_bytes: request.size_bytes,
            sha256: request.sha256,
        }
    }

    pub(super) fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

pub(super) async fn stream_until_complete(
    request: &StreamUploadPlan<'_>,
    peer: &DaemonPeerSnapshot,
    progress: Option<&mut ActiveStreamUploadProgress<'_>>,
) -> Result<RemoteHttpResponse> {
    let mut offset = request.offset;
    let mut retries_at_offset = 0_u8;
    let mut progress = progress;
    loop {
        let path = stream_path(request.upload_id, offset);
        match send_stream_upload_request(
            peer,
            &path,
            request.archive_path,
            offset,
            request.size_bytes,
            request.sha256,
            progress.as_deref_mut(),
        )
        .await
        {
            Ok(response) => match stream_response_state(&response, offset)? {
                StreamResponseState::Terminal => return Ok(response),
                StreamResponseState::Advanced(next_offset) => {
                    offset = next_offset;
                    retries_at_offset = 0;
                }
                StreamResponseState::NoProgress => {
                    retry_same_offset(
                        peer,
                        offset,
                        &mut retries_at_offset,
                        &anyhow!("workspace upload stream response reported no progress"),
                    )?;
                }
            },
            Err(stream_err) => match upload_status(request.target, peer, request.upload_id).await {
                Ok(status) if status.complete => {
                    return Ok(completed_status_response(peer, status));
                }
                Ok(status) if status.offset > offset => {
                    offset = status.offset;
                    retries_at_offset = 0;
                }
                Ok(_) => {
                    retry_same_offset(peer, offset, &mut retries_at_offset, &stream_err)?;
                }
                Err(status_err) => {
                    retry_same_offset(
                        peer,
                        offset,
                        &mut retries_at_offset,
                        &stream_status_error(stream_err, status_err),
                    )?;
                }
            },
        }
    }
}

enum StreamResponseState {
    Terminal,
    Advanced(u64),
    NoProgress,
}

fn stream_response_state(
    response: &RemoteHttpResponse,
    offset: u64,
) -> Result<StreamResponseState> {
    if response.status != 200 {
        return Ok(StreamResponseState::Terminal);
    }
    let parsed = AppendWorkspaceUploadResponse::decode(response.body.as_slice())
        .context("decode workspace upload stream response")?;
    if parsed.complete {
        return Ok(StreamResponseState::Terminal);
    }
    if parsed.offset > offset {
        return Ok(StreamResponseState::Advanced(parsed.offset));
    }
    Ok(StreamResponseState::NoProgress)
}

fn retry_same_offset(
    peer: &DaemonPeerSnapshot,
    offset: u64,
    retries_at_offset: &mut u8,
    err: &anyhow::Error,
) -> Result<()> {
    if *retries_at_offset >= MAX_RETRIES_PER_COMMITTED_OFFSET {
        return Err(DaemonLocalError::retryable_client(format!(
            "workspace upload stream retries exhausted for remote node {} at offset {} after {} retries; last error: {err:#}",
            peer.node_id,
            offset,
            MAX_RETRIES_PER_COMMITTED_OFFSET
        ))
        .into());
    }
    *retries_at_offset += 1;
    Ok(())
}

fn stream_status_error(stream_err: anyhow::Error, status_err: anyhow::Error) -> anyhow::Error {
    anyhow!("stream failed: {stream_err:#}; upload status lookup failed: {status_err:#}")
}

fn stream_path(upload_id: &str, offset: u64) -> String {
    format!("/v2/workspaces/uploads/{upload_id}/stream?offset={offset}")
}
