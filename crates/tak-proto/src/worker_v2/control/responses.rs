use anyhow::{Result, bail};
use base64::Engine;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::{
    AckAttemptRequest, AckAttemptResponse, CancelAttemptResponse, MAX_CONTROL_BYTES,
    OutputChunkRequest, OutputChunkResponse, validation,
};

pub fn encode_cancel_response(response: &CancelAttemptResponse) -> Result<Vec<u8>> {
    validation::response(
        response.protocol_version,
        &response.fencing_token,
        &response.fencing_token,
    )?;
    encode(response)
}

pub fn decode_cancel_response(bytes: &[u8], fence: &str) -> Result<CancelAttemptResponse> {
    let response: CancelAttemptResponse = decode(bytes)?;
    validation::response(response.protocol_version, &response.fencing_token, fence)?;
    Ok(response)
}

pub fn encode_output_chunk_response(response: &OutputChunkResponse) -> Result<Vec<u8>> {
    validate_chunk(response, None)?;
    encode(response)
}

pub fn decode_output_chunk_response(
    bytes: &[u8],
    request: &OutputChunkRequest,
) -> Result<OutputChunkResponse> {
    let response = decode(bytes)?;
    validate_chunk(&response, Some(request))?;
    Ok(response)
}

pub fn encode_ack_response(response: &AckAttemptResponse) -> Result<Vec<u8>> {
    validate_ack(response, None)?;
    encode(response)
}

pub fn decode_ack_response(
    bytes: &[u8],
    request: &AckAttemptRequest,
) -> Result<AckAttemptResponse> {
    let response = decode(bytes)?;
    validate_ack(&response, Some(request))?;
    Ok(response)
}

fn validate_chunk(
    response: &OutputChunkResponse,
    request: Option<&OutputChunkRequest>,
) -> Result<()> {
    let fence = request.map_or(response.fencing_token.as_str(), |value| {
        value.identity.fencing_token.as_str()
    });
    validation::response(response.protocol_version, &response.fencing_token, fence)?;
    if !validation::identifier(&response.artifact_id)
        || request.is_some_and(|value| {
            value.artifact_id != response.artifact_id || value.offset != response.offset
        })
        || !validation::digest(&response.chunk_sha256)
    {
        bail!("worker output chunk response is invalid");
    }
    let chunk = base64::engine::general_purpose::STANDARD.decode(&response.chunk_base64)?;
    if format!("{:x}", Sha256::digest(&chunk)) != response.chunk_sha256
        || request.is_some_and(|value| chunk.len() > value.max_bytes as usize)
    {
        bail!("worker output chunk digest is invalid");
    }
    Ok(())
}

fn validate_ack(response: &AckAttemptResponse, request: Option<&AckAttemptRequest>) -> Result<()> {
    let fence = request.map_or(response.fencing_token.as_str(), |value| {
        value.identity.fencing_token.as_str()
    });
    validation::response(response.protocol_version, &response.fencing_token, fence)?;
    if !response.acknowledged
        || !validation::digest(&response.terminal_digest)
        || request.is_some_and(|value| value.terminal_digest != response.terminal_digest)
    {
        bail!("worker terminal acknowledgement is invalid");
    }
    Ok(())
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > MAX_CONTROL_BYTES {
        bail!("worker control response exceeds the protocol limit");
    }
    Ok(bytes)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > MAX_CONTROL_BYTES {
        bail!("worker control response exceeds the protocol limit");
    }
    Ok(serde_json::from_slice(bytes)?)
}
