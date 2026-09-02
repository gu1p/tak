use anyhow::{Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

mod attempt;
mod cache;
mod control;
mod observation;
mod process;

pub use attempt::{
    DispatchAttemptRequest, WorkerAttemptIdentity, WorkerAttemptPayload, WorkerWorkspace,
    WorkerWorkspaceOverlay, WorkerWorkspaceReuse, decode_dispatch_request, encode_dispatch_request,
    payload_digest,
};
pub use cache::{
    WorkspaceCacheDisposition, WorkspaceCacheProbeRequest, WorkspaceCacheResponse,
    WorkspaceCacheUploadRequest, decode_cache_probe_request, decode_cache_response,
    decode_cache_upload_request, encode_cache_probe_request, encode_cache_response,
    encode_cache_upload_request,
};
pub use control::{
    AckAttemptRequest, AckAttemptResponse, CancelAttemptRequest, CancelAttemptResponse,
    CancelDisposition, ObserveAttemptRequest, OutputChunkRequest, OutputChunkResponse,
    decode_ack_request, decode_ack_response, decode_cancel_request, decode_cancel_response,
    decode_observe_request, decode_output_chunk_request, decode_output_chunk_response,
    encode_ack_request, encode_ack_response, encode_cancel_request, encode_cancel_response,
    encode_observe_request, encode_output_chunk_request, encode_output_chunk_response,
};
pub use observation::{
    DispatchAttemptResponse, DispatchDisposition, MAX_OBSERVE_EVENTS, ObserveAttemptResponse,
    WorkerAttemptEvent, WorkerAttemptState, WorkerOutputArtifact, WorkerOutputStream,
    WorkerTerminal, WorkerTerminalOutcome, decode_dispatch_response, decode_observe_response,
    decode_observe_response_page, encode_dispatch_response, encode_observe_response,
};
pub use process::{
    INCOMPLETE_PROCESS_OBSERVATIONS, WorkerProcessObservation, bounded_process_observations,
};

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_IDENTITY_BYTES: usize = 64 * 1024;
pub const MAX_DISPLAY_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_SNAPSHOT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerDisplayPayload {
    protocol_version: u16,
    payload_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerIdentity {
    pub protocol_version: u16,
    pub node_id: String,
    pub display_name: String,
    pub base_url: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResources {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub execution_slots: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerSnapshot {
    pub protocol_version: u16,
    pub node_id: String,
    pub healthy: bool,
    pub sampled_at_ms: u64,
    pub capacity: WorkerResources,
    pub usage: WorkerResources,
    pub queue_depth: u32,
    pub cached_content: Vec<String>,
    pub processes: Vec<WorkerProcessObservation>,
}

pub fn encode_identity(identity: &WorkerIdentity) -> Result<Vec<u8>> {
    validate_identity(identity)?;
    let encoded = serde_json::to_vec(identity)?;
    if encoded.len() > MAX_IDENTITY_BYTES {
        bail!("worker identity exceeds the protocol limit");
    }
    Ok(encoded)
}

pub fn decode_identity(bytes: &[u8]) -> Result<WorkerIdentity> {
    if bytes.len() > MAX_IDENTITY_BYTES {
        bail!("worker identity exceeds the protocol limit");
    }
    let identity: WorkerIdentity = serde_json::from_slice(bytes)?;
    validate_identity(&identity)?;
    Ok(identity)
}

pub fn encode_display_payload(payload: &[u8]) -> Result<Vec<u8>> {
    if payload.len() > MAX_DISPLAY_PAYLOAD_BYTES {
        bail!("worker display payload exceeds the protocol limit");
    }
    Ok(serde_json::to_vec(&WorkerDisplayPayload {
        protocol_version: PROTOCOL_VERSION,
        payload_base64: STANDARD.encode(payload),
    })?)
}

pub fn decode_display_payload(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() > MAX_DISPLAY_PAYLOAD_BYTES.saturating_mul(4) / 3 + 1024 {
        bail!("worker display payload exceeds the protocol limit");
    }
    let envelope: WorkerDisplayPayload = serde_json::from_slice(bytes)?;
    if envelope.protocol_version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    let payload = STANDARD.decode(envelope.payload_base64)?;
    if payload.len() > MAX_DISPLAY_PAYLOAD_BYTES {
        bail!("worker display payload exceeds the protocol limit");
    }
    Ok(payload)
}

pub fn encode_snapshot(snapshot: &WorkerSnapshot) -> Result<Vec<u8>> {
    validate_snapshot(snapshot)?;
    let encoded = serde_json::to_vec(snapshot)?;
    if encoded.len() > MAX_SNAPSHOT_BYTES {
        bail!("worker snapshot exceeds the protocol limit");
    }
    Ok(encoded)
}

pub fn decode_snapshot(bytes: &[u8]) -> Result<WorkerSnapshot> {
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        bail!("worker snapshot exceeds the protocol limit");
    }
    let snapshot: WorkerSnapshot = serde_json::from_slice(bytes)?;
    validate_snapshot(&snapshot)?;
    Ok(snapshot)
}

fn validate_snapshot(snapshot: &WorkerSnapshot) -> Result<()> {
    if snapshot.protocol_version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    if snapshot.node_id.trim().is_empty()
        || snapshot.node_id.len() > 128
        || snapshot.node_id.chars().any(char::is_control)
    {
        bail!("worker node id is invalid");
    }
    if snapshot.capacity.execution_slots == 0
        || snapshot.usage.cpu_millis > snapshot.capacity.cpu_millis
        || snapshot.usage.memory_bytes > snapshot.capacity.memory_bytes
        || snapshot.usage.execution_slots > snapshot.capacity.execution_slots
    {
        bail!("worker resource snapshot is invalid");
    }
    process::validate(&snapshot.processes)?;
    Ok(())
}

fn validate_identity(identity: &WorkerIdentity) -> Result<()> {
    if identity.protocol_version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    if identity.node_id.trim().is_empty()
        || identity.node_id.len() > 128
        || identity.node_id.chars().any(char::is_control)
    {
        bail!("worker node id is invalid");
    }
    if identity.display_name.trim().is_empty() || identity.base_url.trim().is_empty() {
        bail!("worker identity metadata is incomplete");
    }
    if !matches!(identity.transport.as_str(), "direct" | "tor") {
        bail!("worker identity transport is invalid");
    }
    Ok(())
}
