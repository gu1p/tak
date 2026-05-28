use std::collections::BTreeMap;

use tak_core::remote_inventory::RemoteRecord;

use super::{PeerEntry, PeerSnapshot, PeerState};

pub(super) fn reconcile_peer(peers: &mut BTreeMap<String, PeerEntry>, remote: &RemoteRecord) {
    match peers.get_mut(&remote.node_id) {
        Some(entry) if peer_identity_changed(entry, remote) => {
            *entry = entry_for_remote(remote);
        }
        Some(entry) => {
            entry.snapshot.display_name = remote.display_name.clone();
            entry.snapshot.pools = remote.pools.clone();
            entry.snapshot.tags = remote.tags.clone();
            entry.snapshot.capabilities = remote.capabilities.clone();
        }
        None => {
            peers.insert(remote.node_id.clone(), entry_for_remote(remote));
        }
    }
}

pub(super) fn peer_identity_changed(entry: &PeerEntry, remote: &RemoteRecord) -> bool {
    entry.snapshot.endpoint != remote.base_url
        || entry.snapshot.transport != remote.transport
        || entry.bearer_token != remote.bearer_token
}

fn entry_for_remote(remote: &RemoteRecord) -> PeerEntry {
    PeerEntry {
        snapshot: PeerSnapshot {
            node_id: remote.node_id.clone(),
            display_name: remote.display_name.clone(),
            transport: remote.transport.clone(),
            endpoint: remote.base_url.clone(),
            state: PeerState::Connecting,
            last_heartbeat_ms: None,
            last_successful_connection_ms: None,
            last_error_summary: None,
            active_job_count: None,
            queue_depth: None,
            resource_summary: None,
            protocol_version: None,
            heartbeat_rtt_ms: None,
            reconnect_attempts: 0,
            pools: remote.pools.clone(),
            tags: remote.tags.clone(),
            capabilities: remote.capabilities.clone(),
        },
        bearer_token: remote.bearer_token.clone(),
        consecutive_failures: 0,
        next_heartbeat_due_ms: 0,
    }
}
