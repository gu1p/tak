use std::time::Duration;

use anyhow::{Context, Result};
use arti_client::TorClient;

use super::StrictRemoteTarget;

use crate::default_client_tor_config;

const DEFAULT_TOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(1);

pub(super) fn tor_connect_timeout() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_PROBE_TIMEOUT_MS",
        "TAK_TOR_PROBE_TIMEOUT_MS",
        DEFAULT_TOR_CONNECT_TIMEOUT,
    )
}

pub(super) fn tor_connect_retry_delay() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_PROBE_BACKOFF_MS",
        "TAK_TOR_PROBE_BACKOFF_MS",
        DEFAULT_TOR_CONNECT_RETRY_DELAY,
    )
}

fn env_duration_ms(test_name: &str, live_name: &str, default: Duration) -> Duration {
    std::env::var(test_name)
        .ok()
        .or_else(|| std::env::var(live_name).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

pub(super) async fn shared_tor_client(
    target: &StrictRemoteTarget,
) -> Result<TorClient<tor_rtcompat::PreferredRuntime>> {
    static CELL: std::sync::OnceLock<
        tokio::sync::OnceCell<TorClient<tor_rtcompat::PreferredRuntime>>,
    > = std::sync::OnceLock::new();
    let client = CELL
        .get_or_init(tokio::sync::OnceCell::const_new)
        .get_or_try_init(|| async move {
            let deadline = tokio::time::Instant::now() + tor_connect_timeout();
            let config = default_client_tor_config().with_context(|| {
                format!(
                    "infra error: remote node {} unavailable at {}",
                    target.node_id, target.endpoint
                )
            })?;

            loop {
                match TorClient::create_bootstrapped(config.clone()).await {
                    Ok(client) => return Ok(client),
                    Err(error) if tokio::time::Instant::now() >= deadline => {
                        return Err(error).with_context(|| {
                            format!(
                                "infra error: remote node {} unavailable at {}",
                                target.node_id, target.endpoint
                            )
                        });
                    }
                    Err(_) => {}
                }

                tokio::time::sleep(tor_connect_retry_delay()).await;
            }
        })
        .await?;
    Ok(client.clone())
}
