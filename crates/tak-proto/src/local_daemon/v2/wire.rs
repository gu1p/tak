use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use tak_core::v2::{EnvironmentValue, RemoteRequirements, ResolvedRun, RunSubmission};

use super::identifier::is_valid_identifier;
use super::request_validation::{
    valid_digest, valid_invite, valid_node_ids, valid_remote_path, valid_requirements,
};
use super::{MAX_WORKSPACE_CHUNK_BYTES, Operation, PROTOCOL_VERSION, Request};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRequest {
    protocol_version: u64,
    request_id: String,
    operation: WireOperation,
}

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum WireOperation {
    GetDaemonStatus {},
    PreviewRemote {
        invite: String,
    },
    AddRemote {
        invite: String,
    },
    ListRemotes {},
    RemoveRemote {
        node_id: String,
    },
    GetRemoteStatus {
        node_ids: Vec<String>,
    },
    ReadRemote {
        node_id: String,
        path: String,
    },
    ResolveRemoteCandidates {
        requirements: RemoteRequirements,
    },
    SubmitRun {
        idempotency_key: String,
        run: Box<ResolvedRun>,
        environment_values: Vec<EnvironmentValue>,
    },
    UploadWorkspace {
        run_id: String,
        workspace_fingerprint: String,
        archive_size: u64,
        offset: u64,
        chunk_base64: String,
    },
    CommitRun {
        run_id: String,
    },
    ListRuns {},
    GetRun {
        run_id: String,
    },
    AttachRun {
        run_id: String,
        after_event: u64,
    },
    CancelRun {
        run_id: String,
    },
    GetOutputManifest {
        run_id: String,
    },
    GetOutputChunk {
        artifact_id: String,
        offset: u64,
        max_bytes: u32,
    },
}

pub(super) fn decode_strict(raw: &str) -> Option<Request> {
    let wire = serde_json::from_str::<WireRequest>(raw).ok()?;
    if wire.protocol_version != PROTOCOL_VERSION || !is_valid_identifier(&wire.request_id) {
        return None;
    }
    let operation = match wire.operation {
        WireOperation::GetDaemonStatus {} => Operation::GetDaemonStatus {},
        WireOperation::PreviewRemote { invite } if valid_invite(&invite) => {
            Operation::PreviewRemote { invite }
        }
        WireOperation::AddRemote { invite } if valid_invite(&invite) => {
            Operation::AddRemote { invite }
        }
        WireOperation::ListRemotes {} => Operation::ListRemotes {},
        WireOperation::RemoveRemote { node_id } if is_valid_identifier(&node_id) => {
            Operation::RemoveRemote { node_id }
        }
        WireOperation::GetRemoteStatus { node_ids } if valid_node_ids(&node_ids) => {
            Operation::GetRemoteStatus { node_ids }
        }
        WireOperation::ReadRemote { node_id, path }
            if is_valid_identifier(&node_id) && valid_remote_path(&path) =>
        {
            Operation::ReadRemote { node_id, path }
        }
        WireOperation::ResolveRemoteCandidates { requirements }
            if valid_requirements(&requirements) =>
        {
            Operation::ResolveRemoteCandidates { requirements }
        }
        WireOperation::SubmitRun {
            idempotency_key,
            run,
            environment_values,
        } => {
            let submission = RunSubmission::new(idempotency_key, *run, environment_values).ok()?;
            Operation::SubmitRun {
                idempotency_key: submission.idempotency_key,
                run: Box::new(submission.run),
                environment_values: submission.environment_values,
            }
        }
        WireOperation::UploadWorkspace {
            run_id,
            workspace_fingerprint,
            archive_size,
            offset,
            chunk_base64,
        } if is_valid_identifier(&run_id) && valid_digest(&workspace_fingerprint) => {
            let chunk = STANDARD.decode(chunk_base64).ok()?;
            if chunk.is_empty()
                || chunk.len() > MAX_WORKSPACE_CHUNK_BYTES
                || offset > archive_size
                || offset.saturating_add(chunk.len() as u64) > archive_size
            {
                return None;
            }
            Operation::UploadWorkspace {
                run_id,
                workspace_fingerprint,
                archive_size,
                offset,
                chunk,
            }
        }
        WireOperation::CommitRun { run_id } if is_valid_identifier(&run_id) => {
            Operation::CommitRun { run_id }
        }
        WireOperation::ListRuns {} => Operation::ListRuns {},
        WireOperation::GetRun { run_id } if is_valid_identifier(&run_id) => {
            Operation::GetRun { run_id }
        }
        WireOperation::AttachRun {
            run_id,
            after_event,
        } if is_valid_identifier(&run_id) => Operation::AttachRun {
            run_id,
            after_event,
        },
        WireOperation::CancelRun { run_id } if is_valid_identifier(&run_id) => {
            Operation::CancelRun { run_id }
        }
        WireOperation::GetOutputManifest { run_id } if is_valid_identifier(&run_id) => {
            Operation::GetOutputManifest { run_id }
        }
        WireOperation::GetOutputChunk {
            artifact_id,
            offset,
            max_bytes,
        } if is_valid_identifier(&artifact_id)
            && max_bytes > 0
            && max_bytes as usize <= MAX_WORKSPACE_CHUNK_BYTES =>
        {
            Operation::GetOutputChunk {
                artifact_id,
                offset,
                max_bytes,
            }
        }
        WireOperation::PreviewRemote { .. }
        | WireOperation::AddRemote { .. }
        | WireOperation::RemoveRemote { .. }
        | WireOperation::GetRemoteStatus { .. }
        | WireOperation::ReadRemote { .. }
        | WireOperation::ResolveRemoteCandidates { .. }
        | WireOperation::UploadWorkspace { .. }
        | WireOperation::CommitRun { .. }
        | WireOperation::GetRun { .. }
        | WireOperation::AttachRun { .. }
        | WireOperation::CancelRun { .. }
        | WireOperation::GetOutputManifest { .. }
        | WireOperation::GetOutputChunk { .. } => return None,
    };
    Some(Request {
        request_id: wire.request_id,
        operation,
    })
}
