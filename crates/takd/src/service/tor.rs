use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::sleep;

use crate::agent::{
    TorRecoveryBackoff, TransportHealth, arti_cache_dir, arti_state_dir, read_config,
    write_transport_health,
};
use crate::daemon::remote::SubmitAttemptStore;
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

mod health;
mod live;
mod monitor;
mod probe;
mod rend;
mod startup_policy;
mod test_bind;

use health::tor_recovery_config;
use live::serve_live_tor_session;
use test_bind::{RetryableTestBindStartupFailure, serve_test_bind_session};

pub(super) async fn serve_tor_agent(
    config_root: &Path,
    state_root: &Path,
    store: SubmitAttemptStore,
) -> Result<()> {
    let config = read_config(config_root)?;
    let runtime = TorHiddenServiceRuntimeConfig {
        nickname: config.hidden_service_nickname.clone(),
        state_dir: arti_state_dir(state_root),
        cache_dir: arti_cache_dir(state_root),
    };
    let recovery = tor_recovery_config();
    let mut relaunch_backoff =
        TorRecoveryBackoff::new(recovery.initial_backoff, recovery.max_backoff);
    let mut last_base_url = config.base_url.clone();

    if last_base_url.is_some() {
        write_transport_health(
            state_root,
            &TransportHealth::recovering(
                last_base_url.clone(),
                Some("re-establishing takd onion service".to_string()),
            ),
        )?;
    } else {
        write_transport_health(state_root, &TransportHealth::pending(None))?;
    }

    if let Some(bind_addr) = test_tor_hidden_service_bind_addr() {
        loop {
            match serve_test_bind_session(
                config_root,
                state_root,
                &config,
                &runtime.nickname,
                store.clone(),
                &bind_addr,
            )
            .await
            {
                Ok(exit) => {
                    write_transport_health(
                        state_root,
                        &TransportHealth::recovering(
                            Some(exit.base_url.clone()),
                            Some(exit.reason.clone()),
                        ),
                    )?;
                    let delay = relaunch_backoff.next_delay();
                    tracing::warn!(
                        "restarting takd tor test-bind session after {}ms: {}",
                        delay.as_millis(),
                        exit.reason
                    );
                    sleep(delay).await;
                }
                Err(err) => {
                    if err
                        .downcast_ref::<RetryableTestBindStartupFailure>()
                        .is_none()
                    {
                        return Err(err);
                    }
                    let detail = format!("{err:#}");
                    write_transport_health(
                        state_root,
                        &TransportHealth::recovering(last_base_url.clone(), Some(detail.clone())),
                    )?;
                    let delay = relaunch_backoff.next_delay();
                    tracing::warn!(
                        "retrying takd tor test-bind startup after {}ms: {}",
                        delay.as_millis(),
                        detail
                    );
                    sleep(delay).await;
                }
            }
        }
    }

    loop {
        match serve_live_tor_session(
            config_root,
            state_root,
            &config,
            &runtime,
            store.clone(),
            &recovery,
        )
        .await
        {
            Ok(exit) => {
                last_base_url = Some(exit.base_url.clone());
                write_transport_health(
                    state_root,
                    &TransportHealth::recovering(
                        Some(exit.base_url.clone()),
                        Some(exit.reason.clone()),
                    ),
                )?;
                let delay = relaunch_backoff.next_delay();
                tracing::warn!(
                    "restarting takd onion service after {}ms: {}",
                    delay.as_millis(),
                    exit.reason
                );
                sleep(delay).await;
            }
            Err(err) => {
                let detail = format!("{err:#}");
                write_transport_health(
                    state_root,
                    &TransportHealth::recovering(last_base_url.clone(), Some(detail.clone())),
                )?;
                let delay = relaunch_backoff.next_delay();
                tracing::warn!(
                    "retrying takd onion service bootstrap after {}ms: {}",
                    delay.as_millis(),
                    detail
                );
                sleep(delay).await;
            }
        }
    }
}

pub(super) fn onion_service_config(
    nickname: &str,
) -> Result<arti_client::config::onion_service::OnionServiceConfig> {
    let nickname = nickname
        .trim()
        .parse()
        .with_context(|| format!("invalid tor hidden-service nickname `{nickname}`"))?;
    arti_client::config::onion_service::OnionServiceConfigBuilder::default()
        .nickname(nickname)
        .build()
        .context("invalid onion service config for takd")
}

pub(super) fn test_tor_hidden_service_bind_addr() -> Option<String> {
    std::env::var("TAKD_TEST_TOR_HS_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn ready_config(
    config: &crate::agent::AgentConfig,
    base_url: &str,
) -> crate::agent::AgentConfig {
    let mut ready = config.clone();
    ready.base_url = Some(base_url.to_string());
    ready
}

pub(super) fn tor_client_config(
    runtime: &TorHiddenServiceRuntimeConfig,
) -> Result<arti_client::config::TorClientConfig> {
    arti_client::config::TorClientConfigBuilder::from_directories(
        &runtime.state_dir,
        &runtime.cache_dir,
    )
    .build()
    .context("invalid Arti client configuration for takd hidden service")
}

pub(super) struct TorSessionExit {
    pub(super) base_url: String,
    pub(super) reason: String,
}
