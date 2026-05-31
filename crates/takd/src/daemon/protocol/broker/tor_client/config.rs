use super::*;
use anyhow::anyhow;
use arti_client::TorClientConfig;
use std::path::Path;

pub(super) fn default_client_tor_config(state_root: Option<PathBuf>) -> Result<TorClientConfig> {
    let state_root = match state_root {
        Some(path) => path,
        None => client_state_root()?,
    };
    let arti_root = broker_arti_root(&state_root);
    Ok(
        arti_client::config::TorClientConfigBuilder::from_directories(
            arti_root.join("state"),
            arti_root.join("cache"),
        )
        .build()?,
    )
}

// The outbound broker keeps its own Arti state/cache, deliberately separate from
// the hidden-service client's `<root>/arti` directory (see `agent::arti_state_dir`).
// Two TorClients cannot share one state directory: the second loses the on-disk
// lock and drops to read-only mode, where it never finishes bootstrap. That left
// the broker unable to dial any onion, so every heartbeat timed out before a dial
// even started and peers stayed permanently `unreachable`.
fn broker_arti_root(state_root: &Path) -> PathBuf {
    state_root.join("arti-client")
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

// The hyper HTTP/2 client handshake is a local preface+SETTINGS flush (it does
// not wait for the peer's SETTINGS), so it completes in ~0ms even over a cold
// onion circuit — the onion *dial* has its own `tor_connect_timeout` budget. A
// handshake timeout therefore means a wedged write path, not "peer prefers h1".
const DEFAULT_HTTP2_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
// How long an HTTP/1.1 "pin" (learned when an h2 attempt fell back to a working
// h1) is honored before we re-probe h2: long enough to spare a genuinely h1-only
// legacy peer a doomed h2 dial every 15s heartbeat, short enough that a transient
// h2 miss cannot permanently poison an h2-capable peer.
const DEFAULT_HTTP1_PIN_TTL: Duration = Duration::from_secs(60);

pub(super) fn http2_handshake_timeout() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_HTTP2_HANDSHAKE_MS",
        "TAK_TOR_HTTP2_HANDSHAKE_MS",
    )
    .unwrap_or(DEFAULT_HTTP2_HANDSHAKE_TIMEOUT)
}

pub(super) fn http1_pin_ttl() -> Duration {
    env_duration_ms("TAK_TEST_TOR_HTTP1_PIN_TTL_MS", "TAK_TOR_HTTP1_PIN_TTL_MS")
        .unwrap_or(DEFAULT_HTTP1_PIN_TTL)
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

#[cfg(test)]
mod config_tests;
