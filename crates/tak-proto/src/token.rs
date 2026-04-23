use anyhow::{Context, Result, anyhow};
use base64::Engine;
use prost::Message;

use crate::RemoteTokenPayload;

const PREFIX: &str = "takd:v1:";
const TOR_INVITE_PREFIX: &str = "takd:tor:";
const TOR_INVITE_CHECKSUM_LEN: usize = 5;
const CROCKFORD_BASE32: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CRC32C_POLY: u32 = 0x82F63B78;

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

pub fn encode_tor_invite(base_url: &str) -> Result<String> {
    let host = canonical_onion_host(base_url)?;
    let checksum = encode_tor_invite_checksum(host.as_bytes());
    Ok(format!("{TOR_INVITE_PREFIX}{host}:{checksum}"))
}

pub fn decode_tor_invite(value: &str) -> Result<String> {
    let payload = value
        .strip_prefix(TOR_INVITE_PREFIX)
        .ok_or_else(|| anyhow!("tor invite must start with `{TOR_INVITE_PREFIX}`"))?;
    let (host, checksum) = payload
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("tor invite is missing a checksum"))?;
    let host = canonical_onion_host(host)?;
    let expected = encode_tor_invite_checksum(host.as_bytes());
    if !checksum.eq_ignore_ascii_case(&expected) {
        return Err(anyhow!(
            "tor invite checksum mismatch: expected {expected}, got {}",
            checksum.trim()
        ));
    }
    Ok(format!("http://{host}"))
}

fn canonical_onion_host(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("tor invite requires a .onion host"));
    }

    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);
    if without_scheme.is_empty() {
        return Err(anyhow!("tor invite requires a .onion host"));
    }
    if without_scheme.contains(['/', '?', '#', '@', ':']) {
        return Err(anyhow!(
            "tor invite must contain only a .onion host without port or path"
        ));
    }
    if without_scheme.chars().any(char::is_whitespace) {
        return Err(anyhow!("tor invite host contains whitespace"));
    }

    let host = without_scheme.to_ascii_lowercase();
    if !host.ends_with(".onion") {
        return Err(anyhow!("tor invite host must end with `.onion`"));
    }
    if host == ".onion" {
        return Err(anyhow!("tor invite requires a .onion host"));
    }
    Ok(host)
}

fn encode_tor_invite_checksum(bytes: &[u8]) -> String {
    let value = crc32c(bytes) & 0x01ff_ffff;
    let mut encoded = String::with_capacity(TOR_INVITE_CHECKSUM_LEN);
    for shift in (0..TOR_INVITE_CHECKSUM_LEN).rev() {
        let index = ((value >> (shift * 5)) & 0x1f) as usize;
        encoded.push(CROCKFORD_BASE32[index] as char);
    }
    encoded
}

pub(crate) fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (CRC32C_POLY & mask);
        }
    }
    !crc
}
