use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use safelog::DisplayRedacted;
use tokio::sync::mpsc;

use crate::agent::{
    TorRecoveryBackoff, TorRecoveryTracker, TransportHealth, persist_ready_base_url,
    write_transport_health,
};
use crate::daemon::remote::SubmitAttemptStore;
use crate::daemon::transport::TorHiddenServiceRuntimeConfig;

use super::health::{TorRecoveryConfig, startup_session_timeout};
use super::live_state::{mark_transport_ready, mark_transport_recovering, pending_context};
use super::monitor::{TorHealthEvent, handle_health_event, run_periodic_self_probe};
use super::probe;
use super::rend::spawn_rend_request;
use super::startup_policy::startup_probe_retry_policy;
use super::{TorSessionExit, onion_service_config, tor_client_config};

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
    let tor_client = arti_client::TorClient::create_bootstrapped(tor_client_config(runtime)?)
        .await
        .context("failed to bootstrap embedded Arti for takd hidden service")?;
    let startup_probe_retry_policy = startup_probe_retry_policy();
    let startup_timeout = startup_session_timeout(startup_probe_retry_policy.timeout);
    let mut startup_backoff =
        TorRecoveryBackoff::new(recovery.initial_backoff, recovery.max_backoff);

    'launch: loop {
        let Some((running_service, rend_requests)) = tor_client
            .launch_onion_service(onion_service_config(&runtime.nickname)?)
            .context("failed to launch takd onion service via embedded Arti")?
        else {
            bail!("takd onion service launch was skipped because the service is disabled");
        };
        let base_url = running_service
            .onion_address()
            .map(|value| format!("http://{}", value.display_unredacted()))
            .ok_or_else(|| anyhow!("takd onion service did not expose an onion address"))?;
        let context = pending_context(config, &base_url);
        persist_ready_base_url(config_root, state_root, &base_url)?;
        write_transport_health(
            state_root,
            &TransportHealth::pending(Some(base_url.clone())),
        )?;

        let (health_tx, mut health_rx) = mpsc::unbounded_channel();
        let readiness_client = tor_client.isolated_client();
        let readiness_base_url = base_url.clone();
        let readiness_token = config.bearer_token.clone();
        let readiness = probe::wait_for_tor_hidden_service_startup(
            &readiness_client,
            &readiness_base_url,
            &readiness_token,
            startup_timeout,
            startup_probe_retry_policy.backoff,
        );
        tokio::pin!(readiness);
        futures::pin_mut!(rend_requests);

        loop {
            tokio::select! {
                ready = &mut readiness => {
                    match ready {
                        Ok(()) => {
                            mark_transport_ready(&context, state_root, &base_url)?;
                            crate::daemon::remote::spawn_remote_cleanup_janitor(
                                context.shared_status_state()
                            );
                            tracing::info!("takd remote v1 onion service ready at {base_url}");
                            break;
                        }
                        Err(err) => {
                            mark_transport_recovering(&context, state_root, &base_url, format!("{err:#}"))?;
                            let delay = startup_backoff.next_delay();
                            tracing::warn!(
                                "relaunching takd onion service on existing Tor client after {}ms: {err:#}",
                                delay.as_millis()
                            );
                            drop(running_service);
                            tokio::time::sleep(delay).await;
                            continue 'launch;
                        }
                    }
                }
                maybe_rend_request = rend_requests.next() => {
                    let Some(rend_request) = maybe_rend_request else {
                        let delay = startup_backoff.next_delay();
                        tracing::warn!(
                            "relaunching takd onion service on existing Tor client after {}ms: request stream ended before readiness",
                            delay.as_millis()
                        );
                        drop(running_service);
                        tokio::time::sleep(delay).await;
                        continue 'launch;
                    };
                    spawn_rend_request(
                        rend_request,
                        store.clone(),
                        context.clone(),
                        health_tx.clone(),
                    );
                }
                maybe_event = health_rx.recv() => {
                    if matches!(maybe_event, Some(TorHealthEvent::ProbeSucceeded)) {
                        mark_transport_ready(&context, state_root, &base_url)?;
                        crate::daemon::remote::spawn_remote_cleanup_janitor(
                            context.shared_status_state()
                        );
                        tracing::info!("takd remote v1 onion service ready at {base_url}");
                        break;
                    }
                }
            }
        }

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
                        mark_transport_ready(&context, state_root, &base_url)?;
                    }
                    if let Some(reason) = handle_health_event(&mut tracker, event) {
                        health_task.abort();
                        return Ok(TorSessionExit { base_url, reason });
                    }
                }
            }
        }
    }
}
