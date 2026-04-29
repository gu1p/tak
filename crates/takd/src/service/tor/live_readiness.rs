use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tor_hsservice::status::OnionServiceStatusStream;
use tor_hsservice::{RendRequest, RunningOnionService};
use tor_rtcompat::Runtime;

use crate::agent::{
    AgentConfig, TorRecoveryBackoff, persist_advertised_base_url, persist_ready_base_url,
};
use crate::daemon::remote::{RemoteNodeContext, SubmitAttemptStore};

use super::live_readiness_support::{
    readiness_probe, record_pending_arti_status, record_self_probe_failure, relaunch_after_delay,
    should_relaunch,
};
use super::live_state::mark_transport_ready;
use super::monitor::TorHealthEvent;
use super::rend::spawn_rend_request;
use super::startup_policy::TorStartupProbeRetryPolicy;
use super::status_detail::hidden_service_probe_gate;

pub(super) struct LiveReadinessContext<'a, R>
where
    R: Runtime + Send + Sync,
{
    pub(super) config_root: &'a std::path::Path,
    pub(super) state_root: &'a std::path::Path,
    pub(super) config: &'a AgentConfig,
    pub(super) tor_client: &'a arti_client::TorClient<R>,
    pub(super) base_url: &'a str,
    pub(super) context: &'a RemoteNodeContext,
    pub(super) store: &'a SubmitAttemptStore,
    pub(super) startup_timeout: Duration,
    pub(super) startup_policy: &'a TorStartupProbeRetryPolicy,
    pub(super) startup_backoff: &'a mut TorRecoveryBackoff,
}

pub(super) enum StartupReadiness<S> {
    Ready(ReadyLiveTorService<S>),
    Relaunch,
}

pub(super) struct ReadyLiveTorService<S> {
    pub(super) _running_service: Arc<RunningOnionService>,
    pub(super) rend_requests: Pin<Box<S>>,
    pub(super) service_status_events: Pin<Box<OnionServiceStatusStream>>,
    pub(super) health_tx: mpsc::UnboundedSender<TorHealthEvent>,
    pub(super) health_rx: mpsc::UnboundedReceiver<TorHealthEvent>,
}

pub(super) async fn wait_until_live_tor_ready<R, S>(
    params: LiveReadinessContext<'_, R>,
    running_service: Arc<RunningOnionService>,
    rend_requests: S,
) -> Result<StartupReadiness<S>>
where
    R: Runtime + Send + Sync,
    S: Stream<Item = RendRequest>,
{
    persist_advertised_base_url(params.config_root, params.base_url)?;
    let mut service_status = running_service.status();
    let mut service_state = service_status.state();
    record_pending_arti_status(&params, &service_status)?;

    let (health_tx, mut health_rx) = mpsc::unbounded_channel();
    let readiness_client = params.tor_client.isolated_client();
    let readiness_base_url = params.base_url.to_string();
    let readiness_token = params.config.bearer_token.clone();
    let mut readiness = readiness_probe(
        &readiness_client,
        &readiness_base_url,
        &readiness_token,
        &params,
    );
    let mut rend_requests = Box::pin(rend_requests);
    let mut service_status_events = Box::pin(running_service.status_events());

    loop {
        tokio::select! {
            ready = &mut readiness, if hidden_service_probe_gate(service_state).allows_probe() => {
                match ready {
                    Ok(()) => {
                        persist_ready_base_url(params.config_root, params.state_root, params.base_url)?;
                        mark_transport_ready(params.context, params.state_root, params.base_url)?;
                        crate::daemon::remote::spawn_remote_cleanup_janitor(
                            params.context.clone(),
                            params.store.clone(),
                        );
                        tracing::info!("takd remote v1 onion service ready at {}", params.base_url);
                        return Ok(StartupReadiness::Ready(ReadyLiveTorService {
                            _running_service: running_service,
                            rend_requests,
                            service_status_events,
                            health_tx,
                            health_rx,
                        }));
                    }
                    Err(err) => {
                        let detail = format!("{err:#}");
                        record_self_probe_failure(&params, service_state, &service_status, &detail)?;
                        if should_relaunch(&detail, service_state) {
                            relaunch_after_delay(params.startup_backoff, &detail).await;
                            return Ok(StartupReadiness::Relaunch);
                        }
                        tracing::warn!("takd onion service still pending after self-probe: {detail}");
                        readiness = readiness_probe(
                            &readiness_client,
                            &readiness_base_url,
                            &readiness_token,
                            &params,
                        );
                    }
                }
            }
            maybe_status = service_status_events.next() => {
                let Some(status) = maybe_status else {
                    continue;
                };
                service_status = status;
                service_state = service_status.state();
                record_pending_arti_status(&params, &service_status)?;
                if hidden_service_probe_gate(service_state).requires_relaunch() {
                    let detail = format!("Arti onion-service state={service_state:?}");
                    relaunch_after_delay(params.startup_backoff, &detail).await;
                    return Ok(StartupReadiness::Relaunch);
                }
            }
            maybe_rend_request = rend_requests.next() => {
                let Some(rend_request) = maybe_rend_request else {
                    relaunch_after_delay(
                        params.startup_backoff,
                        "request stream ended before readiness",
                    )
                    .await;
                    return Ok(StartupReadiness::Relaunch);
                };
                spawn_rend_request(
                    rend_request,
                    params.store.clone(),
                    params.context.clone(),
                    health_tx.clone(),
                );
            }
            maybe_event = health_rx.recv() => {
                if matches!(maybe_event, Some(TorHealthEvent::ProbeSucceeded)) {
                    persist_ready_base_url(params.config_root, params.state_root, params.base_url)?;
                    mark_transport_ready(params.context, params.state_root, params.base_url)?;
                    crate::daemon::remote::spawn_remote_cleanup_janitor(
                        params.context.clone(),
                        params.store.clone(),
                    );
                    tracing::info!("takd remote v1 onion service ready at {}", params.base_url);
                    return Ok(StartupReadiness::Ready(ReadyLiveTorService {
                        _running_service: running_service,
                        rend_requests,
                        service_status_events,
                        health_tx,
                        health_rx,
                    }));
                }
            }
        }
    }
}
