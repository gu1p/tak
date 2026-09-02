use serde::{Deserialize, Serialize};

use super::WorkerAttemptIdentity;

mod requests;
mod responses;
mod validation;

pub use requests::{
    decode_ack_request, decode_cancel_request, decode_observe_request, decode_output_chunk_request,
    encode_ack_request, encode_cancel_request, encode_observe_request, encode_output_chunk_request,
};
pub use responses::{
    decode_ack_response, decode_cancel_response, decode_output_chunk_response, encode_ack_response,
    encode_cancel_response, encode_output_chunk_response,
};

pub(super) const MAX_CONTROL_BYTES: usize = 512 * 1024;
pub(super) const MAX_OUTPUT_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveAttemptRequest {
    pub protocol_version: u16,
    pub identity: WorkerAttemptIdentity,
    pub after_event: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelAttemptRequest {
    pub protocol_version: u16,
    pub identity: WorkerAttemptIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputChunkRequest {
    pub protocol_version: u16,
    pub identity: WorkerAttemptIdentity,
    pub artifact_id: String,
    pub offset: u64,
    pub max_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AckAttemptRequest {
    pub protocol_version: u16,
    pub identity: WorkerAttemptIdentity,
    pub terminal_digest: String,
    pub run_terminal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelDisposition {
    Requested,
    Duplicate,
    AlreadyTerminal,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelAttemptResponse {
    pub protocol_version: u16,
    pub fencing_token: String,
    pub disposition: CancelDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputChunkResponse {
    pub protocol_version: u16,
    pub fencing_token: String,
    pub artifact_id: String,
    pub offset: u64,
    pub chunk_base64: String,
    pub chunk_sha256: String,
    pub eof: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AckAttemptResponse {
    pub protocol_version: u16,
    pub fencing_token: String,
    pub terminal_digest: String,
    pub acknowledged: bool,
}
