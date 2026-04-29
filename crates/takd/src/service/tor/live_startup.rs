use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::Stream;
use tor_rtcompat::{PreferredRuntime, Runtime};

use crate::agent::AgentConfig;
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

use super::{onion_service_config, tor_client_config};

pub(super) async fn bootstrap_tor_client(
    config: &AgentConfig,
    runtime: &TorHiddenServiceRuntimeConfig,
) -> Result<arti_client::TorClient<PreferredRuntime>> {
    record_startup_detail(
        "arti bootstrap",
        1,
        Duration::ZERO,
        Duration::ZERO,
        config.base_url.clone(),
        "bootstrapping embedded Arti for takd hidden service",
    );
    let started = Instant::now();
    let client_config = match tor_client_config(runtime) {
        Ok(config) => config,
        Err(err) => {
            record_startup_detail(
                "arti bootstrap",
                1,
                started.elapsed(),
                Duration::ZERO,
                config.base_url.clone(),
                format!("invalid Arti client configuration: {err:#}"),
            );
            return Err(err);
        }
    };
    match arti_client::TorClient::create_bootstrapped(client_config).await {
        Ok(client) => Ok(client),
        Err(err) => {
            record_startup_detail(
                "arti bootstrap",
                1,
                started.elapsed(),
                Duration::ZERO,
                config.base_url.clone(),
                format!("{err:#}"),
            );
            Err(err).context("failed to bootstrap embedded Arti for takd hidden service")
        }
    }
}

pub(super) fn launch_live_onion_service<R>(
    config: &AgentConfig,
    runtime: &TorHiddenServiceRuntimeConfig,
    tor_client: &arti_client::TorClient<R>,
    startup_timeout: Duration,
) -> Result<
    Option<(
        Arc<tor_hsservice::RunningOnionService>,
        impl Stream<Item = tor_hsservice::RendRequest> + use<R>,
    )>,
>
where
    R: Runtime,
{
    record_startup_detail(
        "onion launch",
        1,
        Duration::ZERO,
        startup_timeout,
        config.base_url.clone(),
        "launching takd onion service through embedded Arti",
    );
    let onion_config = match onion_service_config(&runtime.nickname) {
        Ok(config) => config,
        Err(err) => {
            record_startup_detail(
                "onion launch",
                1,
                Duration::ZERO,
                startup_timeout,
                config.base_url.clone(),
                format!("{err:#}"),
            );
            return Err(err);
        }
    };
    match tor_client.launch_onion_service(onion_config) {
        Ok(launched) => Ok(launched),
        Err(err) => {
            record_startup_detail(
                "onion launch",
                1,
                Duration::ZERO,
                startup_timeout,
                config.base_url.clone(),
                format!("{err:#}"),
            );
            Err(err).context("failed to launch takd onion service via embedded Arti")
        }
    }
}

pub(super) fn record_startup_detail(
    stage: &str,
    attempt: u32,
    elapsed: Duration,
    timeout: Duration,
    base_url: Option<String>,
    detail: impl Into<String>,
) {
    let target = base_url
        .as_deref()
        .map(|base_url| format!(" for {base_url}"))
        .unwrap_or_default();
    tracing::info!(
        "Tor startup {stage} attempt {attempt} after {}ms of {}ms{target}: {}",
        elapsed.as_millis(),
        timeout.as_millis(),
        detail.into()
    );
}
