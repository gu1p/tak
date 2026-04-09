use anyhow::{Context, Result, anyhow, bail};

pub fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
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
    let authority = without_scheme[..authority_end].trim();
    if authority.is_empty() {
        bail!("missing host:port");
    }
    if authority.contains(':') {
        return Ok(authority.to_string());
    }
    Ok(format!(
        "{authority}:{}",
        if scheme == "https" { "443" } else { "80" }
    ))
}

pub fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    let socket_addr = endpoint_socket_addr(endpoint)?;
    let (host, raw_port) = socket_addr
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("missing host:port"))?;
    Ok((
        host.to_string(),
        raw_port
            .parse::<u16>()
            .with_context(|| format!("invalid port `{raw_port}`"))?,
    ))
}
