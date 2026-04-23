use anyhow::{Result, anyhow, bail};

use crate::token::{crc32c, decode_tor_invite, encode_tor_invite};

mod base_conversion;
mod dictionary;
mod onion_v3;

const DATA_WORD_COUNT: usize = 18;
pub const TOR_INVITE_WORD_COUNT: usize = DATA_WORD_COUNT + 1;
const CHECKSUM_PREFIX: &[u8] = b"takd-onion-words-v1";

pub fn encode_tor_invite_words(invite: &str) -> Result<String> {
    let base_url = decode_tor_invite(invite)
        .map_err(|_| anyhow!("tor invite words require a takd:tor: invite"))?;
    let onion_bytes = onion_v3::onion_bytes_from_base_url(&base_url)?;
    let data_indices = base_conversion::encode_word_indices(&onion_bytes, DATA_WORD_COUNT)?;
    let checksum = checksum_word_index(&onion_bytes)?;
    let list = dictionary::word_list();
    let mut words = data_indices
        .into_iter()
        .map(|index| list[index as usize])
        .collect::<Vec<_>>();
    words.push(list[checksum as usize]);
    Ok(words.join(" "))
}

pub fn decode_tor_invite_words(words: &str) -> Result<String> {
    let indices = dictionary::lookup_word_indices(words, TOR_INVITE_WORD_COUNT)?;
    let onion_bytes = base_conversion::decode_word_indices(
        &indices[..DATA_WORD_COUNT],
        onion_v3::ONION_BYTES_LEN,
    )?;
    ensure_checksum_word(indices[DATA_WORD_COUNT], &onion_bytes)?;
    onion_v3::ensure_v3_onion_bytes(&onion_bytes)?;
    let label = onion_v3::encode_onion_label(&onion_bytes);
    encode_tor_invite(&format!("http://{label}.onion"))
}

fn checksum_word_index(onion_bytes: &[u8]) -> Result<u16> {
    let mut payload = Vec::with_capacity(CHECKSUM_PREFIX.len() + onion_bytes.len());
    payload.extend_from_slice(CHECKSUM_PREFIX);
    payload.extend_from_slice(onion_bytes);
    let index = (crc32c(&payload) as usize) % dictionary::word_list().len();
    u16::try_from(index).map_err(|_| anyhow!("word list length should fit in u16"))
}

fn ensure_checksum_word(index: u16, onion_bytes: &[u8]) -> Result<()> {
    let expected = checksum_word_index(onion_bytes)?;
    if index != expected {
        bail!("tor invite word checksum mismatch");
    }
    Ok(())
}
