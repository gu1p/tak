use anyhow::Result;
use std::path::Path;
use tokio::time::sleep;

use crate::agent::{AgentConfig, TorRecoveryBackoff, TransportHealth, write_transport_health};
use crate::daemon::remote::SubmitAttemptStore;
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;
use crate::service::control::AgentControlState;

use super::health::TorRecoveryConfig;
use super::live::serve_live_tor_session;
use super::test_bind::{RetryableTestBindStartupFailure, serve_test_bind_session};

pub(super) struct TorLoopContext<'a> {
    pub(super) config_root: &'a Path,
    pub(super) state_root: &'a Path,
    pub(super) config: &'a AgentConfig,
    pub(super) runtime: &'a TorHiddenServiceRuntimeConfig,
    pub(super) store: SubmitAttemptStore,
    pub(super) control_state: AgentControlState,
}

pub(super) async fn run_test_bind_loop(
    context: TorLoopContext<'_>,
    bind_addr: String,
    mut relaunch_backoff: TorRecoveryBackoff,
    last_base_url: Option<String>,
) -> Result<()> {
    loop {
        match serve_test_bind_session(
            context.config_root,
            context.state_root,
            context.config,
            &context.runtime.nickname,
            context.store.clone(),
            &bind_addr,
            context.control_state.clone(),
        )
        .await
        {
            Ok(exit) => {
                write_transport_health(
                    context.state_root,
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
                    context.state_root,
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

pub(super) async fn run_live_loop(
    context: TorLoopContext<'_>,
    recovery: &TorRecoveryConfig,
    mut relaunch_backoff: TorRecoveryBackoff,
    mut last_base_url: Option<String>,
) -> Result<()> {
    loop {
        match serve_live_tor_session(
            context.config_root,
            context.state_root,
            context.config,
            context.runtime,
            context.store.clone(),
            recovery,
            context.control_state.clone(),
        )
        .await
        {
            Ok(exit) => {
                last_base_url = Some(exit.base_url.clone());
                write_transport_health(
                    context.state_root,
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
                    context.state_root,
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
