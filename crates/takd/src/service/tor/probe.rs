use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use endpoint::{endpoint_host_port, endpoint_socket_addr};
use tokio::time::sleep;
use tor_rtcompat::Runtime;

use attempt::{probe_worker_identity, record_startup_failure, run_with_attempt_timeout};
use health_detail::{log_probe_progress, record_probe_failure};
use startup_failure::{StartupTorClientRestart, StartupTorFailureTracker, startup_probe_error};

use super::startup_policy::CappedExponentialBackoff;

mod attempt;
mod endpoint;
mod health_detail;
mod http_client;
mod startup_failure;

const MAX_PROBE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(15);

pub(super) struct HiddenServiceStartupProbeOptions<'a> {
    timeout: Duration,
    initial_backoff: Duration,
    max_backoff: Duration,
    detail_state_root: Option<&'a Path>,
    startup_failure_threshold: u32,
}

impl<'a> HiddenServiceStartupProbeOptions<'a> {
    pub(super) fn new(
        timeout: Duration,
        initial_backoff: Duration,
        max_backoff: Duration,
        detail_state_root: Option<&'a Path>,
        startup_failure_threshold: u32,
    ) -> Self {
        Self {
            timeout,
            initial_backoff,
            max_backoff,
            detail_state_root,
            startup_failure_threshold,
        }
    }

    fn without_detail(timeout: Duration, backoff: Duration) -> Self {
        Self::new(timeout, backoff, backoff, None, u32::MAX)
    }
}

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
    wait_for_tor_hidden_service_startup_with_options(
        tor_client,
        base_url,
        bearer_token,
        HiddenServiceStartupProbeOptions::without_detail(timeout, backoff),
    )
    .await
}

pub(super) async fn wait_for_tor_hidden_service_startup_with_options<R>(
    tor_client: &arti_client::TorClient<R>,
    base_url: &str,
    bearer_token: &str,
    options: HiddenServiceStartupProbeOptions<'_>,
) -> Result<()>
where
    R: Runtime,
{
    let started_at = Instant::now();
    let deadline = Instant::now() + options.timeout;
    let (host, port) = endpoint_host_port(base_url)?;
    let authority = endpoint_socket_addr(base_url)?;
    let mut last_error = anyhow!("hidden service startup probe failed before a response");
    let mut startup_failures = StartupTorFailureTracker::new(options.startup_failure_threshold);
    let mut attempt = 0_u32;
    let mut backoff = CappedExponentialBackoff::new(options.initial_backoff, options.max_backoff);

    loop {
        attempt = attempt.saturating_add(1);
        log_probe_progress(
            base_url,
            "self-probe connect",
            attempt,
            started_at,
            options.timeout,
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
                    options.timeout,
                    "probing /v2/worker/identity through takd onion service",
                );
                match run_with_attempt_timeout(
                    deadline,
                    MAX_PROBE_ATTEMPT_TIMEOUT,
                    "probe takd hidden-service startup endpoint",
                    probe_worker_identity(Box::new(stream), &authority, bearer_token, base_url),
                )
                .await
                {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        let detail = format!("{err:#}");
                        record_probe_failure(
                            options.detail_state_root,
                            base_url,
                            "self-probe http",
                            attempt,
                            started_at,
                            options.timeout,
                            &detail,
                        );
                        record_startup_failure(&mut startup_failures, &detail)?;
                        last_error = err;
                    }
                }
            }
            Err(err) => {
                let detail = format!("{err:#}");
                record_probe_failure(
                    options.detail_state_root,
                    base_url,
                    "self-probe connect",
                    attempt,
                    started_at,
                    options.timeout,
                    &detail,
                );
                record_startup_failure(&mut startup_failures, &detail)?;
                last_error = err;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        sleep(
            backoff
                .next_backoff()
                .min(deadline.saturating_duration_since(Instant::now())),
        )
        .await;
    }

    Err(startup_probe_error(
        last_error,
        startup_failures.observed_tor_failure(),
        base_url,
        options.timeout,
    ))
}

pub(super) fn requires_tor_client_restart(err: &anyhow::Error) -> bool {
    err.downcast_ref::<StartupTorClientRestart>().is_some()
}

mod http_connection_cleanup_tests;
#[cfg(test)]
mod http_response_tests;
#[cfg(test)]
mod http_response_truncated_body_tests;
#[cfg(test)]
mod tests;
