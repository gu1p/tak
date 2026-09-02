use anyhow::{Result, bail};
use serde::{Serialize, de::DeserializeOwned};

use super::{
    AckAttemptRequest, CancelAttemptRequest, MAX_CONTROL_BYTES, ObserveAttemptRequest,
    OutputChunkRequest, validation,
};

pub fn encode_observe_request(request: &ObserveAttemptRequest) -> Result<Vec<u8>> {
    validation::request(request.protocol_version, &request.identity)?;
    encode(request)
}

pub fn decode_observe_request(bytes: &[u8]) -> Result<ObserveAttemptRequest> {
    let request: ObserveAttemptRequest = decode(bytes)?;
    validation::request(request.protocol_version, &request.identity)?;
    Ok(request)
}

pub fn encode_cancel_request(request: &CancelAttemptRequest) -> Result<Vec<u8>> {
    validation::request(request.protocol_version, &request.identity)?;
    encode(request)
}

pub fn decode_cancel_request(bytes: &[u8]) -> Result<CancelAttemptRequest> {
    let request: CancelAttemptRequest = decode(bytes)?;
    validation::request(request.protocol_version, &request.identity)?;
    Ok(request)
}

pub fn encode_output_chunk_request(request: &OutputChunkRequest) -> Result<Vec<u8>> {
    validation::output_request(request)?;
    encode(request)
}

pub fn decode_output_chunk_request(bytes: &[u8]) -> Result<OutputChunkRequest> {
    let request = decode(bytes)?;
    validation::output_request(&request)?;
    Ok(request)
}

pub fn encode_ack_request(request: &AckAttemptRequest) -> Result<Vec<u8>> {
    validation::ack_request(request)?;
    encode(request)
}

pub fn decode_ack_request(bytes: &[u8]) -> Result<AckAttemptRequest> {
    let request = decode(bytes)?;
    validation::ack_request(&request)?;
    Ok(request)
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_CONTROL_BYTES {
        bail!("worker control request exceeds the protocol limit");
    }
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_CONTROL_BYTES {
        bail!("worker control request exceeds the protocol limit");
    }
    Ok(serde_json::from_slice(bytes)?)
}
