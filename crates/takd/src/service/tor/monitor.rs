use tokio::sync::mpsc;
use tor_rtcompat::Runtime;

use crate::agent::TorRecoveryTracker;

use super::health::TorRecoveryConfig;
use super::probe;

pub(super) enum TorHealthEvent {
    ProbeSucceeded,
    Failure(String),
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum TorHealthTransition {
    Ready,
    KeepReady,
    Recovering(String),
}

pub(super) fn handle_health_event(
    tracker: &mut TorRecoveryTracker,
    event: TorHealthEvent,
) -> TorHealthTransition {
    match event {
        TorHealthEvent::ProbeSucceeded => {
            tracker.record_success();
            TorHealthTransition::Ready
        }
        TorHealthEvent::Failure(reason) => {
            if tracker.record_failure() {
                TorHealthTransition::Recovering(format!(
                    "{reason}; relaunching after {} consecutive transport failures",
                    tracker.consecutive_failures()
                ))
            } else {
                TorHealthTransition::KeepReady
            }
        }
    }
}

pub(super) async fn run_periodic_self_probe<R>(
    tor_client: arti_client::TorClient<R>,
    base_url: String,
    bearer_token: String,
    recovery: TorRecoveryConfig,
    health_tx: mpsc::UnboundedSender<TorHealthEvent>,
) where
    R: Runtime + Send + Sync + 'static,
{
    let mut ticker = tokio::time::interval(recovery.probe_interval);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let event = match probe::wait_for_tor_hidden_service_startup(
            &tor_client,
            &base_url,
            &bearer_token,
            recovery.probe_timeout,
            recovery.probe_backoff,
        )
        .await
        {
            Ok(()) => TorHealthEvent::ProbeSucceeded,
            Err(err) => TorHealthEvent::Failure(format!("self-probe failed: {err:#}")),
        };
        if health_tx.send(event).is_err() {
            return;
        }
    }
}

#[path = "monitor_tests.rs"]
mod monitor_tests;
