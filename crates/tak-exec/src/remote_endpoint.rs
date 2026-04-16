use std::env;

use anyhow::{Result, bail};
use tak_core::model::RemoteTransportKind;

pub(crate) fn remote_protocol_bearer_token<'a>(
    node_id: &str,
    bearer_token: &'a str,
    transport_kind: RemoteTransportKind,
) -> Result<Option<&'a str>> {
    let token = bearer_token.trim();
    if token.is_empty() {
        return if transport_kind == RemoteTransportKind::Tor {
            Ok(None)
        } else {
            bail!("infra error: remote node {} bearer token is empty", node_id)
        };
    }
    if token.contains(['\r', '\n']) {
        bail!(
            "infra error: remote node {} bearer token contains invalid characters",
            node_id
        );
    }
    Ok(Some(token))
}

pub fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    tak_core::endpoint::endpoint_socket_addr(endpoint).map_err(Into::into)
}

pub fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    tak_core::endpoint::endpoint_host_port(endpoint).map_err(Into::into)
}

pub fn socket_addr_from_host_port(host: &str, port: u16) -> String {
    if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

pub(crate) fn test_tor_onion_dial_addr() -> Option<String> {
    env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
