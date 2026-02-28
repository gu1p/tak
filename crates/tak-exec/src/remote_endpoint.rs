use std::env;

use anyhow::{Context, Result, anyhow, bail};

pub(crate) fn remote_protocol_request_headers(
    node_id: &str,
    service_auth_env: Option<&str>,
) -> Result<String> {
    let mut headers = String::from("X-Tak-Protocol-Version: v1\r\n");

    if let Some(env_name) = service_auth_env {
        let token = env::var(env_name).with_context(|| {
            format!(
                "infra error: remote node {} missing service auth token env {}",
                node_id, env_name
            )
        })?;
        let token = token.trim();
        if token.is_empty() {
            bail!(
                "infra error: remote node {} service auth token env {} is empty",
                node_id,
                env_name
            );
        }
        if token.contains(['\r', '\n']) {
            bail!(
                "infra error: remote node {} service auth token env {} contains invalid characters",
                node_id,
                env_name
            );
        }
        headers.push_str(&format!("X-Tak-Service-Token: {token}\r\n"));
    }

    Ok(headers)
}

pub(crate) fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim();
    let (scheme, without_scheme) = if let Some(value) = trimmed.strip_prefix("http://") {
        ("http", value)
    } else if let Some(value) = trimmed.strip_prefix("https://") {
        ("https", value)
    } else {
        ("", trimmed)
    };

    let authority_end = without_scheme
        .find(['/', '?', '#'])
        .unwrap_or(without_scheme.len());
    let authority_with_userinfo = without_scheme[..authority_end].trim();
    let authority = authority_with_userinfo
        .rsplit_once('@')
        .map_or(authority_with_userinfo, |(_, value)| value)
        .trim();
    if authority.is_empty() {
        bail!("missing host:port");
    }

    if authority.contains(':') {
        return Ok(authority.to_string());
    }

    if scheme.is_empty() {
        bail!("missing port in endpoint authority");
    }

    let default_port = if scheme == "https" { "443" } else { "80" };
    Ok(format!("{authority}:{default_port}"))
}

pub(crate) fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    let socket_addr = endpoint_socket_addr(endpoint)?;
    let (host, raw_port) = socket_addr
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("missing host:port"))?;
    if host.trim().is_empty() {
        bail!("missing host");
    }
    let port = raw_port
        .parse::<u16>()
        .with_context(|| format!("invalid port `{raw_port}`"))?;
    Ok((host.to_string(), port))
}

pub(crate) fn test_tor_onion_dial_addr() -> Option<String> {
    env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
