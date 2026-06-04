use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use tak_core::remote_inventory::{RemoteInventory, load_remote_inventory_at};

use super::{PeerConnectionTarget, PeerManager};
use crate::daemon::protocol::TorBroker;

const INVENTORY_RELOAD_INTERVAL: Duration = Duration::from_secs(1);

impl PeerManager {
    pub fn from_inventory(inventory: RemoteInventory) -> Self {
        let manager = Self::default();
        manager.apply_inventory(inventory);
        manager
    }

    pub fn apply_inventory_result(
        &self,
        inventory: Result<RemoteInventory>,
    ) -> Vec<PeerConnectionTarget> {
        match inventory {
            Ok(inventory) => self.apply_inventory(inventory),
            Err(err) => {
                tracing::warn!("preserving last-good remote inventory: {err:#}");
                Vec::new()
            }
        }
    }

    pub fn apply_inventory(&self, inventory: RemoteInventory) -> Vec<PeerConnectionTarget> {
        let mut state = self.lock_state();
        let local_identity = state.local_identity.clone();
        let next = inventory
            .enabled_tor_remotes()
            // Never adopt the local node as a peer: the local takd is a bridge,
            // and a submit must reach a remote, never loop back to itself.
            .filter(|remote| {
                local_identity.as_ref().is_none_or(|identity| {
                    !identity.matches_peer(&remote.node_id, &remote.base_url)
                })
            })
            .map(|remote| (remote.node_id.clone(), remote))
            .collect::<BTreeMap<_, _>>();
        let mut evicted = evicted_peers(&mut state.peers, &next);
        state
            .placement_assignments
            .retain(|node_id, _| next.contains_key(node_id));
        state
            .round_robin_cursors
            .retain(|node_ids, _| node_ids.iter().all(|node_id| next.contains_key(node_id)));
        for remote in next.values() {
            if let Some(entry) = state.peers.get(&remote.node_id)
                && super::reconcile::peer_identity_changed(entry, remote)
            {
                evicted.push(entry.connection_target());
            }
            super::reconcile::reconcile_peer(&mut state.peers, remote);
        }
        evicted.sort_unstable_by(|left, right| left.node_id.cmp(&right.node_id));
        evicted
    }

    pub fn spawn_inventory_reloader(&self, path: PathBuf) {
        self.spawn_inventory_reloader_inner(path, None);
    }

    pub fn spawn_inventory_reloader_with_broker(&self, path: PathBuf, broker: TorBroker) {
        self.spawn_inventory_reloader_inner(path, Some(broker));
    }

    fn spawn_inventory_reloader_inner(&self, path: PathBuf, broker: Option<TorBroker>) {
        let manager = self.clone();
        tokio::spawn(async move {
            loop {
                let evicted = manager.apply_inventory_result(load_remote_inventory_at(&path));
                evict_broker_sessions(broker.as_ref(), evicted).await;
                tokio::time::sleep(INVENTORY_RELOAD_INTERVAL).await;
            }
        });
    }
}

fn evicted_peers(
    peers: &mut BTreeMap<String, super::PeerEntry>,
    next: &BTreeMap<String, &tak_core::remote_inventory::RemoteRecord>,
) -> Vec<PeerConnectionTarget> {
    let mut evicted = Vec::new();
    for node_id in peers.keys().cloned().collect::<Vec<_>>() {
        if !next.contains_key(&node_id)
            && let Some(entry) = peers.remove(&node_id)
        {
            evicted.push(entry.connection_target());
        }
    }
    evicted
}

async fn evict_broker_sessions(broker: Option<&TorBroker>, evicted: Vec<PeerConnectionTarget>) {
    let Some(broker) = broker else {
        return;
    };
    for target in evicted {
        broker
            .evict_http2_session_for_peer(&target.endpoint, &target.node_id, &target.bearer_token)
            .await;
    }
}
