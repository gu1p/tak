use super::*;
use anyhow::anyhow;
use arti_client::TorClientConfig;

pub(super) fn default_client_tor_config(state_root: Option<PathBuf>) -> Result<TorClientConfig> {
    let state_root = match state_root {
        Some(path) => path,
        None => client_state_root()?,
    };
    Ok(
        arti_client::config::TorClientConfigBuilder::from_directories(
            state_root.join("arti").join("state"),
            state_root.join("arti").join("cache"),
        )
        .build()?,
    )
}

pub(super) async fn create_bootstrapped(
    config: TorClientConfig,
) -> Result<TorClient<tor_rtcompat::PreferredRuntime>> {
    let deadline = tokio::time::Instant::now() + tor_connect_timeout();
    loop {
        match TorClient::create_bootstrapped(config.clone()).await {
            Ok(client) => return Ok(client),
            Err(err) if tokio::time::Instant::now() >= deadline => return Err(err.into()),
            Err(_) => tokio::time::sleep(tor_connect_retry_delay()).await,
        }
    }
}

pub(super) fn test_tor_onion_dial_addr() -> Option<String> {
    std::env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn tor_connect_timeout() -> Duration {
    env_duration_ms("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "TAK_TOR_PROBE_TIMEOUT_MS")
        .unwrap_or(DEFAULT_TOR_CONNECT_TIMEOUT)
}

pub(super) fn tor_connect_retry_delay() -> Duration {
    env_duration_ms("TAK_TEST_TOR_PROBE_BACKOFF_MS", "TAK_TOR_PROBE_BACKOFF_MS")
        .unwrap_or(DEFAULT_TOR_CONNECT_RETRY_DELAY)
}

pub(super) fn socket_addr_from_host_port(host: &str, port: u16) -> String {
    if host.contains(':') && !(host.starts_with('[') && host.ends_with(']')) {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn client_state_root() -> Result<PathBuf> {
    std::env::var("XDG_STATE_HOME")
        .map(|value| PathBuf::from(value).join("tak"))
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/state/tak")))
        .map_err(|_| anyhow!("failed to resolve tak client state root"))
}

fn env_duration_ms(test_name: &str, live_name: &str) -> Option<Duration> {
    std::env::var(test_name)
        .ok()
        .or_else(|| std::env::var(live_name).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
}
