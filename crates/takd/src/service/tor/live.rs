use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use futures::StreamExt;
use safelog::DisplayRedacted;

use crate::agent::{TorRecoveryBackoff, TorRecoveryTracker, persist_ready_base_url};
use crate::daemon::remote::SubmitAttemptStore;
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

use super::TorSessionExit;
use super::health::{TorRecoveryConfig, startup_session_timeout};
use super::live_readiness::{LiveReadinessContext, StartupReadiness, wait_until_live_tor_ready};
use super::live_readiness_support::{current_problem, log_tor_client_bootstrap_status};
use super::live_startup::{bootstrap_tor_client, launch_live_onion_service, record_startup_detail};
use super::live_state::{mark_transport_ready, mark_transport_recovering, pending_context};
use super::monitor::{TorHealthEvent, handle_health_event, run_periodic_self_probe};
use super::rend::spawn_rend_request;
use super::startup_policy::startup_probe_retry_policy;
use super::status_detail::{format_arti_transport_detail, hidden_service_probe_gate};

pub(super) async fn serve_live_tor_session(
    config_root: &std::path::Path,
    state_root: &std::path::Path,
    config: &crate::agent::AgentConfig,
    runtime: &TorHiddenServiceRuntimeConfig,
    store: SubmitAttemptStore,
    recovery: &TorRecoveryConfig,
) -> Result<TorSessionExit> {
    tracing::info!(
        "bootstrapping embedded Arti for takd hidden service nickname {}",
        runtime.nickname
    );
    let tor_client = bootstrap_tor_client(config, runtime).await?;
    log_tor_client_bootstrap_status(&tor_client);
    let startup_probe_retry_policy = startup_probe_retry_policy();
    let startup_timeout = startup_session_timeout(startup_probe_retry_policy.timeout);
    let mut startup_backoff =
        TorRecoveryBackoff::new(recovery.initial_backoff, recovery.max_backoff);

    'launch: loop {
        let launched = launch_live_onion_service(config, runtime, &tor_client, startup_timeout)?;
        let Some((running_service, rend_requests)) = launched else {
            record_startup_detail(
                "onion launch",
                1,
                Duration::ZERO,
                startup_timeout,
                config.base_url.clone(),
                "takd onion service launch was skipped because the service is disabled",
            );
            bail!("takd onion service launch was skipped because the service is disabled");
        };
        let base_url = running_service
            .onion_address()
            .map(|value| format!("http://{}", value.display_unredacted()))
            .ok_or_else(|| anyhow!("takd onion service did not expose an onion address"))?;
        let context = pending_context(config, &base_url, state_root)?;
        let ready = wait_until_live_tor_ready(
            LiveReadinessContext {
                config_root,
                state_root,
                config,
                tor_client: &tor_client,
                base_url: &base_url,
                context: &context,
                store: &store,
                startup_timeout,
                startup_policy: &startup_probe_retry_policy,
                startup_backoff: &mut startup_backoff,
            },
            running_service,
            rend_requests,
        )
        .await?;
        let ready = match ready {
            StartupReadiness::Ready(ready) => ready,
            StartupReadiness::Relaunch => continue 'launch,
            StartupReadiness::RestartTorClient { reason } => {
                return Ok(TorSessionExit { base_url, reason });
            }
        };
        let _running_service = ready._running_service;
        let mut rend_requests = ready.rend_requests;
        let mut service_status_events = ready.service_status_events;
        let health_tx = ready.health_tx;
        let mut health_rx = ready.health_rx;

        let probe_tx = health_tx.clone();
        let health_client = tor_client.isolated_client();
        let health_base_url = base_url.clone();
        let health_token = config.bearer_token.clone();
        let recovery_config = recovery.clone();
        let health_task = tokio::spawn(async move {
            run_periodic_self_probe(
                health_client,
                health_base_url,
                health_token,
                recovery_config,
                probe_tx,
            )
            .await;
        });

        let mut tracker = TorRecoveryTracker::new(recovery.failure_threshold);
        loop {
            tokio::select! {
                maybe_rend_request = rend_requests.next() => {
                    let Some(rend_request) = maybe_rend_request else {
                        health_task.abort();
                        return Ok(TorSessionExit {
                            base_url,
                            reason: "takd onion service request stream ended".to_string(),
                        });
                    };
                    spawn_rend_request(
                        rend_request,
                        store.clone(),
                        context.clone(),
                        health_tx.clone(),
                    );
                }
                maybe_event = health_rx.recv() => {
                    let Some(event) = maybe_event else {
                        health_task.abort();
                        return Ok(TorSessionExit {
                            base_url,
                            reason: "takd onion service health monitor stopped".to_string(),
                        });
                    };
                    if let TorHealthEvent::Failure(reason) = &event {
                        mark_transport_recovering(&context, state_root, &base_url, reason.clone())?;
                    }
                    if matches!(&event, TorHealthEvent::ProbeSucceeded) {
                        persist_ready_base_url(config_root, state_root, &base_url)?;
                        mark_transport_ready(&context, state_root, &base_url)?;
                    }
                    if let Some(reason) = handle_health_event(&mut tracker, event) {
                        health_task.abort();
                        return Ok(TorSessionExit { base_url, reason });
                    }
                }
                maybe_status = service_status_events.next() => {
                    let Some(status) = maybe_status else {
                        continue;
                    };
                    let state = status.state();
                    tracing::info!(
                        "{}",
                        format_arti_transport_detail(
                            &base_url,
                            tor_client.bootstrap_status().to_string(),
                            state,
                            current_problem(&status).as_deref(),
                        )
                    );
                    if hidden_service_probe_gate(state).requires_relaunch() {
                        health_task.abort();
                        return Ok(TorSessionExit {
                            base_url,
                            reason: format!("Arti onion-service state={state:?}"),
                        });
                    }
                }
            }
        }
    }
}
