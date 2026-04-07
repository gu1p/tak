use anyhow::{Context, Result, anyhow};
use base64::Engine;
use prost::Message;

use crate::RemoteTokenPayload;

const PREFIX: &str = "takd:v1:";

pub fn encode_remote_token(payload: &RemoteTokenPayload) -> Result<String> {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.encode_to_vec());
    Ok(format!("{PREFIX}{encoded}"))
}

pub fn decode_remote_token(value: &str) -> Result<RemoteTokenPayload> {
    let payload = value
        .strip_prefix(PREFIX)
        .ok_or_else(|| anyhow!("remote token must start with `{PREFIX}`"))?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .context("decode remote token base64")?;
    RemoteTokenPayload::decode(decoded.as_slice()).context("decode remote token protobuf")
}
