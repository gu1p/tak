use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use tak_core::v2::{EnvironmentValue, ResolvedRun, RunSubmission};

use super::identifier::is_valid_identifier;
use super::{
    MAX_REQUEST_FRAME_BYTES, MAX_WORKSPACE_CHUNK_BYTES, Operation, PROTOCOL_VERSION, Request,
    RequestEncodeError,
};

#[derive(Serialize)]
struct WireRequest<'a> {
    protocol_version: u64,
    request_id: &'a str,
    operation: WireOperation<'a>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WireOperation<'a> {
    SubmitRun {
        idempotency_key: &'a str,
        run: &'a ResolvedRun,
        environment_values: &'a [EnvironmentValue],
    },
    UploadWorkspace {
        run_id: &'a str,
        workspace_fingerprint: &'a str,
        archive_size: u64,
        offset: u64,
        chunk_base64: String,
    },
    CommitRun {
        run_id: &'a str,
    },
    ListRuns {},
    GetRun {
        run_id: &'a str,
    },
    AttachRun {
        run_id: &'a str,
        after_event: u64,
    },
    CancelRun {
        run_id: &'a str,
    },
    GetOutputManifest {
        run_id: &'a str,
    },
    GetOutputChunk {
        artifact_id: &'a str,
        offset: u64,
        max_bytes: u32,
    },
}

pub fn encode_request(request: &Request) -> Result<String, RequestEncodeError> {
    if !is_valid_identifier(&request.request_id) {
        return Err(RequestEncodeError::RequestIdInvalid);
    }
    let operation = encode_operation(&request.operation)?;
    let encoded = serde_json::to_string(&WireRequest {
        protocol_version: PROTOCOL_VERSION,
        request_id: &request.request_id,
        operation,
    })
    .map_err(|_| RequestEncodeError::EncodingFailed)?;
    if encoded.len() > MAX_REQUEST_FRAME_BYTES {
        return Err(RequestEncodeError::FrameTooLarge);
    }
    Ok(encoded)
}

fn encode_operation(operation: &Operation) -> Result<WireOperation<'_>, RequestEncodeError> {
    let encoded = match operation {
        Operation::SubmitRun {
            idempotency_key,
            run,
            environment_values,
        } => {
            RunSubmission::new(
                idempotency_key.clone(),
                (**run).clone(),
                environment_values.clone(),
            )
            .map_err(|_| RequestEncodeError::PayloadInvalid)?;
            WireOperation::SubmitRun {
                idempotency_key,
                run,
                environment_values,
            }
        }
        Operation::UploadWorkspace {
            run_id,
            workspace_fingerprint,
            archive_size,
            offset,
            chunk,
        } => {
            let run_id = valid_run_id(run_id)?;
            if !valid_digest(workspace_fingerprint)
                || chunk.is_empty()
                || chunk.len() > MAX_WORKSPACE_CHUNK_BYTES
                || offset.saturating_add(chunk.len() as u64) > *archive_size
            {
                return Err(RequestEncodeError::PayloadInvalid);
            }
            WireOperation::UploadWorkspace {
                run_id,
                workspace_fingerprint,
                archive_size: *archive_size,
                offset: *offset,
                chunk_base64: STANDARD.encode(chunk),
            }
        }
        Operation::CommitRun { run_id } => WireOperation::CommitRun {
            run_id: valid_run_id(run_id)?,
        },
        Operation::ListRuns {} => WireOperation::ListRuns {},
        Operation::GetRun { run_id } => WireOperation::GetRun {
            run_id: valid_run_id(run_id)?,
        },
        Operation::AttachRun {
            run_id,
            after_event,
        } => WireOperation::AttachRun {
            run_id: valid_run_id(run_id)?,
            after_event: *after_event,
        },
        Operation::CancelRun { run_id } => WireOperation::CancelRun {
            run_id: valid_run_id(run_id)?,
        },
        Operation::GetOutputManifest { run_id } => WireOperation::GetOutputManifest {
            run_id: valid_run_id(run_id)?,
        },
        Operation::GetOutputChunk {
            artifact_id,
            offset,
            max_bytes,
        } => {
            if !is_valid_identifier(artifact_id)
                || *max_bytes == 0
                || *max_bytes as usize > MAX_WORKSPACE_CHUNK_BYTES
            {
                return Err(RequestEncodeError::PayloadInvalid);
            }
            WireOperation::GetOutputChunk {
                artifact_id,
                offset: *offset,
                max_bytes: *max_bytes,
            }
        }
    };
    Ok(encoded)
}

fn valid_run_id(run_id: &str) -> Result<&str, RequestEncodeError> {
    is_valid_identifier(run_id)
        .then_some(run_id)
        .ok_or(RequestEncodeError::RunIdInvalid)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
