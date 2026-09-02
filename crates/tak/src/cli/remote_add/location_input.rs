use anyhow::{Result, bail};
use tak_proto::encode_tor_invite;

pub(super) fn token_from_location_input(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if ["takd:v2:", "takd:v1:", "takd:tor:"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return Ok(trimmed.to_string());
    }
    if trimmed.contains(".onion") {
        return encode_tor_invite(trimmed);
    }
    bail!("paste a takd token or secret Tor invite/address");
}
