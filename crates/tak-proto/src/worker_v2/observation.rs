use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use tak_core::v2::WorkspaceEntry;

mod validation;

use validation::{validate_dispatch, validate_observation, validate_observation_page};

const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_OBSERVE_EVENTS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchDisposition {
    Accepted,
    Duplicate,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchAttemptResponse {
    pub protocol_version: u16,
    pub fencing_token: String,
    pub disposition: DispatchDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerAttemptEvent {
    pub seq: u64,
    pub task_id: String,
    pub stream: WorkerOutputStream,
    pub chunk_base64: String,
    pub chunk_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerOutputArtifact {
    pub artifact_id: String,
    pub producer_task_id: String,
    pub entry: WorkspaceEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTerminalOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerTerminal {
    pub outcome: WorkerTerminalOutcome,
    pub terminal_digest: String,
    pub event_watermark: u64,
    pub outputs: Vec<WorkerOutputArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_engine: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerAttemptState {
    Running,
    Completed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveAttemptResponse {
    pub protocol_version: u16,
    pub fencing_token: String,
    pub state: WorkerAttemptState,
    pub events: Vec<WorkerAttemptEvent>,
    pub next_event: u64,
    pub terminal: Option<WorkerTerminal>,
}

pub fn encode_dispatch_response(response: &DispatchAttemptResponse) -> Result<Vec<u8>> {
    validate_dispatch(response, &response.fencing_token)?;
    encode(response)
}

pub fn decode_dispatch_response(bytes: &[u8], fence: &str) -> Result<DispatchAttemptResponse> {
    let response = decode(bytes)?;
    validate_dispatch(&response, fence)?;
    Ok(response)
}

pub fn encode_observe_response(response: &ObserveAttemptResponse) -> Result<Vec<u8>> {
    validate_observation(response, &response.fencing_token)?;
    encode(response)
}

pub fn decode_observe_response(bytes: &[u8], fence: &str) -> Result<ObserveAttemptResponse> {
    let response = decode(bytes)?;
    validate_observation(&response, fence)?;
    Ok(response)
}

pub fn decode_observe_response_page(
    bytes: &[u8],
    fence: &str,
    after_event: u64,
) -> Result<ObserveAttemptResponse> {
    let response = decode_observe_response(bytes, fence)?;
    validate_observation_page(&response, after_event)?;
    Ok(response)
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        bail!("worker response exceeds the protocol limit");
    }
    Ok(bytes)
}

fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        bail!("worker response exceeds the protocol limit");
    }
    Ok(serde_json::from_slice(bytes)?)
}
