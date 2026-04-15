use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtiSettings {
    pub socks5_addr: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorTransportConfig {
    pub onion_endpoint: String,
    pub service_auth_token: String,
    pub arti: ArtiSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorHiddenServiceRuntimeConfig {
    pub nickname: String,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}

/// Validates Tor transport configuration before any transport/client creation.
///
/// ```compile_fail
/// // Reason: rustdoc links full `takd` runnable doctests, and this crate currently trips a nightly `rust-lld` bus error during link.
/// # use takd::daemon::transport::{ArtiSettings, TorTransportConfig, validate_tor_transport_config};
/// # let _force_compile_fail: () = 1;
/// let config = TorTransportConfig {
///     onion_endpoint: "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion".to_string(),
///     service_auth_token: "service-token-123".to_string(),
///     arti: ArtiSettings {
///         socks5_addr: "127.0.0.1:9150".to_string(),
///         data_dir: "/tmp/tak/arti".to_string(),
///     },
/// };
/// validate_tor_transport_config(&config).unwrap();
/// ```
pub fn validate_tor_transport_config(config: &TorTransportConfig) -> Result<()> {
    ensure_present("onion endpoint", &config.onion_endpoint)?;
    if !is_valid_onion_endpoint(&config.onion_endpoint) {
        bail!("tor onion endpoint must target a .onion host");
    }

    ensure_present("service auth token", &config.service_auth_token)?;
    if config.service_auth_token.chars().any(char::is_whitespace) {
        bail!("tor service auth token contains invalid characters");
    }

    ensure_present("arti socks5 address", &config.arti.socks5_addr)?;
    ensure_present("arti data directory", &config.arti.data_dir)?;
    Ok(())
}

/// Validates and canonicalizes Tor transport configuration values.
///
/// ```compile_fail
/// // Reason: rustdoc links full `takd` runnable doctests, and this crate currently trips a nightly `rust-lld` bus error during link.
/// # use takd::daemon::transport::{ArtiSettings, TorTransportConfig, normalize_tor_transport_config};
/// # let _force_compile_fail: () = 1;
/// let normalized = normalize_tor_transport_config(TorTransportConfig {
///     onion_endpoint: "  http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion  ".to_string(),
///     service_auth_token: " service-token-123 ".to_string(),
///     arti: ArtiSettings {
///         socks5_addr: " 127.0.0.1:9150 ".to_string(),
///         data_dir: " /tmp/tak/arti ".to_string(),
///     },
/// })
/// .unwrap();
/// assert_eq!(normalized.arti.socks5_addr, "127.0.0.1:9150");
/// ```
pub fn normalize_tor_transport_config(config: TorTransportConfig) -> Result<TorTransportConfig> {
    let normalized = TorTransportConfig {
        onion_endpoint: config.onion_endpoint.trim().to_string(),
        service_auth_token: config.service_auth_token.trim().to_string(),
        arti: ArtiSettings {
            socks5_addr: config.arti.socks5_addr.trim().to_string(),
            data_dir: config.arti.data_dir.trim().to_string(),
        },
    };
    validate_tor_transport_config(&normalized)?;
    Ok(normalized)
}

fn ensure_present(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("tor {field} is required");
    }
    Ok(())
}

fn is_valid_onion_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.trim();
    let candidate = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        format!("http://{endpoint}")
    };
    let Ok(parsed) = url::Url::parse(&candidate) else {
        return false;
    };
    parsed
        .host_str()
        .is_some_and(|host| host.ends_with(".onion"))
}
