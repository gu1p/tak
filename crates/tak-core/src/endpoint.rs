use thiserror::Error;
use url::{Host, Url};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EndpointError {
    #[error("missing host:port")]
    MissingHostPort,
    #[error("missing port in endpoint authority")]
    MissingPortInAuthority,
    #[error("missing host")]
    MissingHost,
    #[error("invalid port `{0}`")]
    InvalidPort(String),
    #[error("unsupported endpoint scheme `{0}`")]
    UnsupportedScheme(String),
    #[error("invalid endpoint `{0}`")]
    InvalidEndpoint(String),
}

pub fn endpoint_socket_addr(endpoint: &str) -> Result<String, EndpointError> {
    let parsed = parse_endpoint_url(endpoint)?;
    Ok(format!(
        "{}:{}",
        format_host_for_authority(parsed.host().ok_or(EndpointError::MissingHost)?),
        resolved_port(&parsed)?
    ))
}

pub fn endpoint_host_port(endpoint: &str) -> Result<(String, u16), EndpointError> {
    let parsed = parse_endpoint_url(endpoint)?;
    let host = match parsed.host().ok_or(EndpointError::MissingHost)? {
        Host::Domain(value) => value.to_string(),
        Host::Ipv4(value) => value.to_string(),
        Host::Ipv6(value) => value.to_string(),
    };
    Ok((host, resolved_port(&parsed)?))
}

fn parse_endpoint_url(endpoint: &str) -> Result<Url, EndpointError> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(EndpointError::MissingHostPort);
    }

    let has_scheme = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    if trimmed.contains("://") && !has_scheme {
        let scheme = trimmed.split("://").next().unwrap_or_default();
        return Err(EndpointError::UnsupportedScheme(scheme.to_string()));
    }

    let candidate = if has_scheme {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    };
    let parsed =
        Url::parse(&candidate).map_err(|_| EndpointError::InvalidEndpoint(trimmed.to_string()))?;
    if parsed.host().is_none() {
        return Err(EndpointError::MissingHostPort);
    }
    if !has_scheme && parsed.port().is_none() {
        return Err(EndpointError::MissingPortInAuthority);
    }
    Ok(parsed)
}

fn resolved_port(url: &Url) -> Result<u16, EndpointError> {
    url.port_or_known_default()
        .ok_or(EndpointError::MissingPortInAuthority)
}

fn format_host_for_authority(host: Host<&str>) -> String {
    match host {
        Host::Domain(value) => value.to_string(),
        Host::Ipv4(value) => value.to_string(),
        Host::Ipv6(value) => format!("[{value}]"),
    }
}
