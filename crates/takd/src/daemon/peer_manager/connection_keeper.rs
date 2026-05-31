use std::collections::HashSet;
use std::time::Duration;

use super::{PeerConnectionTarget, PeerEntry, PeerManager};
use crate::daemon::protocol::TorBroker;

// How often the keeper re-checks every peer and redials any dropped link. Short
// so a connection is restored within ~1s of dropping; a healthy link is a cheap
// is_closed() check, so a tight cadence is fine.
const KEEPER_INTERVAL: Duration = Duration::from_secs(1);

impl PeerManager {
    pub fn all_connection_targets(&self) -> Vec<PeerConnectionTarget> {
        let state = self.lock_state();
        state
            .peers
            .values()
            .map(PeerEntry::connection_target)
            .collect()
    }

    /// Eagerly opens and then permanently holds a warm HTTP/2 connection to every
    /// configured peer, redialing within one interval of any drop, so a submit
    /// always lands on an already-open connection instead of cold-dialing.
    ///
    /// ```no_run
    /// // Reason: spawns a background task; needs a tokio runtime and a broker.
    /// # fn demo(peers: &takd::PeerManager, broker: takd::TorBroker) {
    /// peers.spawn_connection_keeper(broker);
    /// # }
    /// ```
    pub fn spawn_connection_keeper(&self, broker: TorBroker) {
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                let targets = manager.all_connection_targets();
                prune_orphan_sessions(&broker, &targets).await;
                for target in targets {
                    warm_one_peer(broker.clone(), target);
                }
                tokio::time::sleep(KEEPER_INTERVAL).await;
            }
        });
    }
}

// Drop pooled connections to peers no longer configured, so a removed peer's
// link (possibly resurrected by an in-flight dial) cannot outlive its inventory.
async fn prune_orphan_sessions(broker: &TorBroker, targets: &[PeerConnectionTarget]) {
    let live_keys = targets
        .iter()
        .map(|target| {
            broker.warm_session_key(&target.endpoint, &target.node_id, &target.bearer_token)
        })
        .collect::<HashSet<String>>();
    broker.retain_warm_sessions(&live_keys).await;
}

// Dial (or confirm) one peer's warm session off the keeper's hot path, so a slow
// onion dial for one peer never delays warming the others.
fn warm_one_peer(broker: TorBroker, target: PeerConnectionTarget) {
    tokio::spawn(async move {
        if let Err(err) = broker
            .ensure_warm_session(&target.endpoint, &target.node_id, &target.bearer_token)
            .await
        {
            tracing::debug!(
                node_id = %target.node_id,
                error = %format!("{err:#}"),
                "keeper could not warm peer connection; will retry"
            );
        }
    });
}
