use tokio::sync::mpsc;
use tor_rtcompat::Runtime;

use crate::agent::TorRecoveryTracker;

use super::health::TorRecoveryConfig;
use super::probe;

pub(super) enum TorHealthEvent {
    ProbeSucceeded,
    Failure(String),
}

pub(super) fn handle_health_event(
    tracker: &mut TorRecoveryTracker,
    event: TorHealthEvent,
) -> Option<String> {
    match event {
        TorHealthEvent::ProbeSucceeded => {
            tracker.record_success();
            None
        }
        TorHealthEvent::Failure(reason) => {
            if tracker.record_failure() {
                Some(format!(
                    "{reason}; relaunching after {} consecutive transport failures",
                    tracker.consecutive_failures()
                ))
            } else {
                None
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
