use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use tor_hsservice::status::{OnionServiceStatus, State};
use tor_rtcompat::Runtime;

use crate::agent::TorRecoveryBackoff;

use super::live_readiness::LiveReadinessContext;
use super::live_state::mark_transport_pending;
use super::probe;
use super::status_detail::{
    SelfProbeRecoveryAction, format_arti_transport_detail, hidden_service_probe_gate,
    self_probe_failure_action,
};

pub(super) type ReadinessProbe<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

pub(super) fn log_tor_client_bootstrap_status<R>(tor_client: &arti_client::TorClient<R>)
where
    R: Runtime,
{
    let status = tor_client.bootstrap_status();
    tracing::info!("Arti bootstrap status for takd hidden service: {status}");
    if let Some(blockage) = status.blocked() {
        tracing::warn!(
            "Arti bootstrap blocked for takd hidden service: {blockage}; status={status}"
        );
    }
}

pub(super) fn readiness_probe<'a, R>(
    client: &'a arti_client::TorClient<R>,
    base_url: &'a str,
    token: &'a str,
    params: &LiveReadinessContext<'a, R>,
) -> ReadinessProbe<'a>
where
    R: Runtime + Send + Sync + 'a,
{
    Box::pin(probe::wait_for_tor_hidden_service_startup_with_detail(
        client,
        base_url,
        token,
        params.startup_timeout,
        params.startup_policy.initial_backoff,
        params.startup_policy.max_backoff,
        Some(params.state_root),
    ))
}

pub(super) fn record_pending_arti_status<R>(
    params: &LiveReadinessContext<'_, R>,
    service_status: &OnionServiceStatus,
) -> Result<()>
where
    R: Runtime,
{
    let detail = format_arti_transport_detail(
        params.base_url,
        params.tor_client.bootstrap_status().to_string(),
        service_status.state(),
        current_problem(service_status).as_deref(),
    );
    tracing::info!("{detail}");
    mark_transport_pending(params.context, params.state_root, params.base_url, detail)
}

pub(super) fn record_self_probe_failure<R>(
    params: &LiveReadinessContext<'_, R>,
    service_state: State,
    service_status: &OnionServiceStatus,
    detail: &str,
) -> Result<()>
where
    R: Runtime,
{
    mark_transport_pending(
        params.context,
        params.state_root,
        params.base_url,
        format!(
            "{}; self-probe failed: {detail}",
            format_arti_transport_detail(
                params.base_url,
                params.tor_client.bootstrap_status().to_string(),
                service_state,
                current_problem(service_status).as_deref(),
            )
        ),
    )
}

pub(super) fn self_probe_recovery_action(
    detail: &str,
    service_state: State,
) -> SelfProbeRecoveryAction {
    if hidden_service_probe_gate(service_state).requires_relaunch() {
        return SelfProbeRecoveryAction::RelaunchService;
    }
    self_probe_failure_action(detail)
}

pub(super) async fn relaunch_after_delay(startup_backoff: &mut TorRecoveryBackoff, detail: &str) {
    let delay = startup_backoff.next_delay();
    tracing::warn!(
        "relaunching takd onion service on existing Tor client after {}ms: {detail}",
        delay.as_millis()
    );
    tokio::time::sleep(delay).await;
}

pub(super) fn current_problem(status: &OnionServiceStatus) -> Option<String> {
    status
        .current_problem()
        .map(|problem| format!("{problem:?}"))
}
