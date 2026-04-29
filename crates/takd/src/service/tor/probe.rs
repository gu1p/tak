use std::future::Future;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tokio::time::{sleep, timeout};
use tor_rtcompat::Runtime;

use health_detail::{log_probe_progress, record_probe_failure};
use http_client::{RemoteStream, send_node_info_request};

mod health_detail;
mod http_client;

const MAX_PROBE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) async fn wait_for_tor_hidden_service_startup<R>(
    tor_client: &arti_client::TorClient<R>,
    base_url: &str,
    bearer_token: &str,
    timeout: Duration,
    backoff: Duration,
) -> Result<()>
where
    R: Runtime,
{
    wait_for_tor_hidden_service_startup_with_detail(
        tor_client,
        base_url,
        bearer_token,
        timeout,
        backoff,
        None,
    )
    .await
}

pub(super) async fn wait_for_tor_hidden_service_startup_with_detail<R>(
    tor_client: &arti_client::TorClient<R>,
    base_url: &str,
    bearer_token: &str,
    timeout: Duration,
    backoff: Duration,
    detail_state_root: Option<&Path>,
) -> Result<()>
where
    R: Runtime,
{
    let started_at = Instant::now();
    let deadline = Instant::now() + timeout;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    let mut last_error = anyhow!("hidden service startup probe failed before a response");
    let mut attempt = 0_u32;

    loop {
        attempt = attempt.saturating_add(1);
        log_probe_progress(
            base_url,
            "self-probe connect",
            attempt,
            started_at,
            timeout,
            "connecting to takd onion service through embedded Arti",
        );
        match run_with_attempt_timeout(
            deadline,
            MAX_PROBE_ATTEMPT_TIMEOUT,
            "connect takd hidden-service startup probe",
            tor_client.connect((host.as_str(), port)),
        )
        .await
        .context("connect takd hidden-service startup probe")
        {
            Ok(stream) => {
                log_probe_progress(
                    base_url,
                    "self-probe http",
                    attempt,
                    started_at,
                    timeout,
                    "probing /v1/node/info through takd onion service",
                );
                match run_with_attempt_timeout(
                    deadline,
                    MAX_PROBE_ATTEMPT_TIMEOUT,
                    "probe takd hidden-service startup endpoint",
                    probe_node_info(Box::new(stream), &authority, bearer_token, base_url),
                )
                .await
                {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        record_probe_failure(
                            detail_state_root,
                            base_url,
                            "self-probe http",
                            attempt,
                            started_at,
                            timeout,
                            format!("{err:#}"),
                        );
                        last_error = err;
                    }
                }
            }
            Err(err) => {
                record_probe_failure(
                    detail_state_root,
                    base_url,
                    "self-probe connect",
                    attempt,
                    started_at,
                    timeout,
                    format!("{err:#}"),
                );
                last_error = err;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        sleep(backoff).await;
    }

    Err(last_error).context(format!(
        "Tor onion service at {base_url} did not become reachable within {}ms during takd startup",
        timeout.as_millis()
    ))
}

async fn probe_node_info(
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<()> {
    let (status, body) = send_node_info_request(stream, authority, bearer_token, base_url).await?;
    if status != 200 {
        bail!("node probe failed with HTTP {status}");
    }
    tak_proto::NodeInfo::decode(body.as_slice()).context("decode node info protobuf")?;
    Ok(())
}

async fn run_with_attempt_timeout<T, F, E>(
    deadline: Instant,
    max_timeout: Duration,
    stage: &str,
    future: F,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let attempt_timeout = deadline
        .saturating_duration_since(Instant::now())
        .min(max_timeout);
    if attempt_timeout.is_zero() {
        bail!("{stage} timed out before the attempt started");
    }
    timeout(attempt_timeout, future)
        .await
        .map_err(|_| anyhow!("{stage} timed out after {}ms", attempt_timeout.as_millis()))?
        .map_err(Into::into)
}

fn endpoint_socket_addr(endpoint: &str) -> Result<String> {
    tak_core::endpoint::endpoint_socket_addr(endpoint).map_err(Into::into)
}

fn endpoint_host_port(endpoint: &str) -> Result<(String, u16)> {
    tak_core::endpoint::endpoint_host_port(endpoint).map_err(Into::into)
}

mod http_connection_cleanup_tests;
#[cfg(test)]
mod http_response_tests;
#[cfg(test)]
mod http_response_truncated_body_tests;
#[cfg(test)]
mod tests;
