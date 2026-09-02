use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tak_core::v2::{
    EnvironmentValue, JobContextManifest, OutputSelector, ResolvedTaskUnit, ResourceRequest,
    WorkspaceDescriptor, WorkspaceEntry,
};

const MAX_DISPATCH_BYTES: usize = 512 * 1024 * 1024;

mod validation;

pub(super) fn validate_identity(identity: &WorkerAttemptIdentity) -> Result<()> {
    validation::validate_identity(identity)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerAttemptIdentity {
    pub run_id: String,
    pub job_id: String,
    pub node_id: String,
    pub authored_attempt: u32,
    pub dispatch_generation: u32,
    pub fencing_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode", deny_unknown_fields)]
pub enum WorkerWorkspaceReuse {
    Private,
    Shared {
        session_id: String,
        affinity_group: String,
    },
    Paths {
        session_id: String,
        paths: Vec<OutputSelector>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerWorkspaceOverlay {
    pub entry: WorkspaceEntry,
    pub content_base64: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerWorkspace {
    pub descriptor: WorkspaceDescriptor,
    pub overlays: Vec<WorkerWorkspaceOverlay>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerAttemptPayload {
    pub workspace: WorkerWorkspace,
    pub workspace_reuse: WorkerWorkspaceReuse,
    pub tasks: Vec<ResolvedTaskUnit>,
    pub environment_values: Vec<EnvironmentValue>,
    pub resources: ResourceRequest,
    pub context_manifest: JobContextManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchAttemptRequest {
    pub protocol_version: u16,
    pub identity: WorkerAttemptIdentity,
    pub payload_digest: String,
    pub payload: WorkerAttemptPayload,
}

pub fn payload_digest(payload: &WorkerAttemptPayload) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(payload)?)
    ))
}

pub fn encode_dispatch_request(request: &DispatchAttemptRequest) -> Result<Vec<u8>> {
    validation::validate(request)?;
    let encoded = serde_json::to_vec(request)?;
    if encoded.len() > MAX_DISPATCH_BYTES {
        bail!("worker dispatch exceeds the protocol limit");
    }
    Ok(encoded)
}

pub fn decode_dispatch_request(bytes: &[u8]) -> Result<DispatchAttemptRequest> {
    if bytes.len() > MAX_DISPATCH_BYTES {
        bail!("worker dispatch exceeds the protocol limit");
    }
    let request = serde_json::from_slice(bytes)?;
    validation::validate(&request)?;
    Ok(request)
}
