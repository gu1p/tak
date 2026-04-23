use anyhow::{Result, anyhow, bail};
use sha3::{Digest, Sha3_256};

pub(super) const ONION_BYTES_LEN: usize = 35;
const ONION_LABEL_LEN: usize = 56;
const ONION_VERSION: u8 = 3;
const ONION_CHECKSUM_PREFIX: &[u8] = b".onion checksum";
const BASE32_ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";

pub(super) fn onion_bytes_from_base_url(base_url: &str) -> Result<[u8; ONION_BYTES_LEN]> {
    let host = base_url
        .trim()
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("tor invite words require a canonical http:// onion url"))?;
    let label = host
        .strip_suffix(".onion")
        .ok_or_else(|| anyhow!("tor invite words require a v3 onion host"))?;
    if label.len() != ONION_LABEL_LEN {
        bail!("tor invite words require a v3 onion host");
    }
    let onion_bytes = decode_onion_label(label)?;
    ensure_v3_onion_bytes(&onion_bytes)?;
    Ok(onion_bytes)
}

pub(super) fn ensure_v3_onion_bytes(onion_bytes: &[u8]) -> Result<()> {
    if onion_bytes.len() != ONION_BYTES_LEN || onion_bytes[34] != ONION_VERSION {
        bail!("tor invite words require a v3 onion host");
    }
    let expected = tor_v3_checksum(&onion_bytes[..32], ONION_VERSION);
    if onion_bytes[32..34] != expected {
        bail!("invalid v3 onion checksum");
    }
    Ok(())
}

pub(super) fn encode_onion_label(onion_bytes: &[u8]) -> String {
    let mut output = String::with_capacity(ONION_LABEL_LEN);
    let mut buffer = 0_u64;
    let mut bit_count = 0_u8;

    for &byte in onion_bytes {
        buffer = (buffer << 8) | u64::from(byte);
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            let index = ((buffer >> bit_count) & 0x1f) as usize;
            output.push(BASE32_ALPHABET[index] as char);
        }
    }

    debug_assert_eq!(output.len(), ONION_LABEL_LEN);
    output
}

fn decode_onion_label(label: &str) -> Result<[u8; ONION_BYTES_LEN]> {
    let mut output = [0_u8; ONION_BYTES_LEN];
    let mut buffer = 0_u64;
    let mut bit_count = 0_u8;
    let mut output_len = 0_usize;

    for ch in label.bytes() {
        buffer = (buffer << 5) | u64::from(base32_value(ch)?);
        bit_count += 5;
        if bit_count < 8 {
            continue;
        }
        bit_count -= 8;
        output[output_len] = ((buffer >> bit_count) & 0xff) as u8;
        output_len += 1;
    }

    if output_len != ONION_BYTES_LEN || bit_count != 0 {
        bail!("tor invite words require a v3 onion host");
    }
    Ok(output)
}

fn tor_v3_checksum(public_key: &[u8], version: u8) -> [u8; 2] {
    let mut hasher = Sha3_256::new();
    hasher.update(ONION_CHECKSUM_PREFIX);
    hasher.update(public_key);
    hasher.update([version]);
    let digest = hasher.finalize();
    [digest[0], digest[1]]
}

fn base32_value(ch: u8) -> Result<u8> {
    match ch {
        b'a'..=b'z' => Ok(ch - b'a'),
        b'2'..=b'7' => Ok(26 + (ch - b'2')),
        _ => bail!("tor invite words require a v3 onion host"),
    }
}
