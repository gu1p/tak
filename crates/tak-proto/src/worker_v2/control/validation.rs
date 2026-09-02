use anyhow::{Result, bail};

use super::{AckAttemptRequest, OutputChunkRequest};
use crate::worker_v2::{PROTOCOL_VERSION, WorkerAttemptIdentity, attempt::validate_identity};

pub(super) fn request(version: u16, identity: &WorkerAttemptIdentity) -> Result<()> {
    if version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    validate_identity(identity)
}

pub(super) fn output_request(request: &OutputChunkRequest) -> Result<()> {
    self::request(request.protocol_version, &request.identity)?;
    if !identifier(&request.artifact_id)
        || request.max_bytes == 0
        || request.max_bytes as usize > super::MAX_OUTPUT_CHUNK_BYTES
    {
        bail!("worker output chunk request is invalid");
    }
    Ok(())
}

pub(super) fn ack_request(request: &AckAttemptRequest) -> Result<()> {
    self::request(request.protocol_version, &request.identity)?;
    if !digest(&request.terminal_digest) {
        bail!("worker terminal digest is invalid");
    }
    Ok(())
}

pub(super) fn response(version: u16, actual_fence: &str, expected_fence: &str) -> Result<()> {
    if version != PROTOCOL_VERSION || !identifier(actual_fence) || actual_fence != expected_fence {
        bail!("worker control response envelope is invalid");
    }
    Ok(())
}

pub(super) fn identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

pub(super) fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
