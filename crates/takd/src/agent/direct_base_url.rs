use anyhow::{Result, anyhow};
use url::{Host, Url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedDirectBaseUrl {
    pub(crate) original: String,
    pub(crate) scheme: String,
    pub(crate) host_for_bind: String,
    pub(crate) port: u16,
}

impl ParsedDirectBaseUrl {
    pub(crate) fn bind_addr(&self) -> String {
        format!("{}:{}", self.host_for_bind, self.port)
    }

    pub(crate) fn canonical_base_url(&self) -> String {
        format!("{}://{}", self.scheme, self.bind_addr())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectBaseUrlError {
    Missing,
    InvalidScheme,
    MissingHost,
    MissingPort,
    UnsupportedComponents,
}

pub(crate) fn parse_direct_base_url(
    base_url: Option<&str>,
) -> std::result::Result<ParsedDirectBaseUrl, DirectBaseUrlError> {
    let base_url = base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(DirectBaseUrlError::Missing)?;
    let parsed = Url::parse(base_url).map_err(|_| DirectBaseUrlError::InvalidScheme)?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(DirectBaseUrlError::InvalidScheme);
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(DirectBaseUrlError::UnsupportedComponents);
    }
    let host_for_bind = match parsed.host().ok_or(DirectBaseUrlError::MissingHost)? {
        Host::Domain(value) => value.to_string(),
        Host::Ipv4(value) => value.to_string(),
        Host::Ipv6(value) => format!("[{value}]"),
    };
    let port = parsed.port().ok_or(DirectBaseUrlError::MissingPort)?;

    Ok(ParsedDirectBaseUrl {
        original: base_url.to_string(),
        scheme: parsed.scheme().to_string(),
        host_for_bind,
        port,
    })
}

pub(crate) fn validate_direct_base_url(base_url: Option<&str>) -> Result<String> {
    parse_direct_base_url(base_url)
        .map(|parsed| parsed.canonical_base_url())
        .map_err(|err| match err {
            DirectBaseUrlError::Missing => anyhow!("base_url is required for direct transport"),
            DirectBaseUrlError::InvalidScheme => {
                anyhow!("base_url must start with http:// or https:// for direct transport")
            }
            DirectBaseUrlError::MissingHost => {
                anyhow!("base_url must include a host for direct transport")
            }
            DirectBaseUrlError::MissingPort => {
                anyhow!("base_url must include a port for direct transport")
            }
            DirectBaseUrlError::UnsupportedComponents => anyhow!(
                "base_url must not include userinfo, path, query, or fragment for direct transport"
            ),
        })
}

#[path = "direct_base_url_tests.rs"]
mod direct_base_url_tests;
