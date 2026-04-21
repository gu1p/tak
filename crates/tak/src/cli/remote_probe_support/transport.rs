use anyhow::{Context, Result, anyhow};
use arti_client::TorClient;
use std::future::Future;
use std::time::{Duration, Instant};
use tak_core::model::RemoteTransportKind;
use tak_exec::{default_client_tor_config, socket_addr_from_host_port};
use tokio::net::TcpStream;
use tokio::time::sleep;

use super::{ProbeAttemptError, RemoteStream};

const DEFAULT_TOR_PROBE_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_PROBE_BACKOFF: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
pub(in crate::cli) struct TorProbeRetryPolicy {
    pub(super) timeout: Duration,
    pub(super) backoff: Duration,
}

#[derive(Clone, Copy)]
pub(in crate::cli) struct TorOnionRetryTexts {
    pub(in crate::cli) build_config: &'static str,
    pub(in crate::cli) bootstrap: &'static str,
    pub(in crate::cli) connect: &'static str,
    pub(in crate::cli) timeout_tail: &'static str,
    pub(in crate::cli) no_retryable_error: &'static str,
}

pub(in crate::cli) async fn connect_remote(
    host: &str,
    port: u16,
    kind: RemoteTransportKind,
) -> Result<RemoteStream> {
    if kind == RemoteTransportKind::Direct || !host.ends_with(".onion") {
        return Ok(Box::new(
            TcpStream::connect(socket_addr_from_host_port(host, port)).await?,
        ));
    }
    if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
        return Ok(Box::new(TcpStream::connect(test_dial_addr).await?));
    }
    let client = TorClient::create_bootstrapped(default_client_tor_config()?).await?;
    Ok(Box::new(client.connect((host, port)).await?))
}

pub(in crate::cli) async fn retry_tor_onion<T, F, Fut>(
    base_url: &str,
    host: &str,
    port: u16,
    texts: TorOnionRetryTexts,
    mut attempt: F,
) -> Result<T>
where
    F: FnMut(RemoteStream) -> Fut,
    Fut: Future<Output = std::result::Result<T, ProbeAttemptError>>,
{
    let retry_policy = tor_probe_retry_policy();
    let deadline = Instant::now() + retry_policy.timeout;
    let test_dial_addr = test_tor_onion_dial_addr();
    let mut tor_client = None;
    let mut last_error = anyhow!(texts.no_retryable_error);

    loop {
        if test_dial_addr.is_none() && tor_client.is_none() {
            let config = default_client_tor_config().context(texts.build_config)?;
            match TorClient::create_bootstrapped(config)
                .await
                .context(texts.bootstrap)
            {
                Ok(client) => tor_client = Some(client),
                Err(err) => {
                    last_error = ProbeAttemptError::retryable(err).into_anyhow();
                    if Instant::now() >= deadline {
                        break;
                    }
                    sleep(retry_policy.backoff).await;
                    continue;
                }
            }
        }

        let stream = if let Some(test_dial_addr) = test_dial_addr.as_deref() {
            TcpStream::connect(test_dial_addr)
                .await
                .context(texts.connect)
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        } else {
            tor_client
                .as_ref()
                .expect("tor client should be initialized before connect")
                .connect((host, port))
                .await
                .context(texts.connect)
                .map(|stream| Box::new(stream) as RemoteStream)
                .map_err(ProbeAttemptError::retryable)
        };

        match stream {
            Ok(stream) => match attempt(stream).await {
                Ok(value) => return Ok(value),
                Err(err) if err.is_retryable() => last_error = err.into_anyhow(),
                Err(err) => return Err(err.into_anyhow()),
            },
            Err(err) => last_error = err.into_anyhow(),
        }

        if Instant::now() >= deadline {
            break;
        }
        sleep(retry_policy.backoff).await;
    }

    Err(last_error).context(format!(
        "Tor onion service at {base_url} did not become reachable within {}ms{}",
        retry_policy.timeout.as_millis(),
        texts.timeout_tail
    ))
}

pub(in crate::cli) fn test_tor_onion_dial_addr() -> Option<String> {
    std::env::var("TAK_TEST_TOR_ONION_DIAL_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(in crate::cli) fn tor_probe_retry_policy() -> TorProbeRetryPolicy {
    TorProbeRetryPolicy {
        timeout: env_duration_ms(
            "TAK_TEST_TOR_PROBE_TIMEOUT_MS",
            "TAK_TOR_PROBE_TIMEOUT_MS",
            DEFAULT_TOR_PROBE_TIMEOUT,
        ),
        backoff: env_duration_ms(
            "TAK_TEST_TOR_PROBE_BACKOFF_MS",
            "TAK_TOR_PROBE_BACKOFF_MS",
            DEFAULT_TOR_PROBE_BACKOFF,
        ),
    }
}

fn env_duration_ms(test_name: &str, live_name: &str, default: Duration) -> Duration {
    env_duration_ms_var(test_name)
        .or_else(|| env_duration_ms_var(live_name))
        .unwrap_or(default)
}

fn env_duration_ms_var(name: &str) -> Option<Duration> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
}
