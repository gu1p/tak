//! Maps the local takd peer snapshot into remote-status result rows.

use std::collections::BTreeSet;

use super::super::{DaemonPeerSnapshot, RemoteRecord, RemoteStatusResult};

pub(super) fn results_from_peers(
    peers: Vec<DaemonPeerSnapshot>,
    node_filters: &[String],
) -> Vec<RemoteStatusResult> {
    let wanted = node_filters
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    let mut results = peers
        .into_iter()
        .filter(|peer| peer.transport == "tor")
        .filter(|peer| wanted.is_empty() || wanted.contains(peer.node_id.as_str()))
        .map(result_from_peer)
        .collect::<Vec<_>>();
    results.sort_unstable_by(|left, right| left.remote.node_id.cmp(&right.remote.node_id));
    results
}

fn result_from_peer(peer: DaemonPeerSnapshot) -> RemoteStatusResult {
    let error = peer_failure_status(&peer).map(str::to_string);
    RemoteStatusResult {
        remote: RemoteRecord {
            node_id: peer.node_id.clone(),
            display_name: peer.display_name.clone(),
            base_url: peer.endpoint.clone(),
            bearer_token: String::new(),
            pools: peer.pools.clone(),
            tags: peer.tags.clone(),
            capabilities: peer.capabilities.clone(),
            transport: peer.transport.clone(),
            enabled: true,
        },
        status: None,
        error,
        peer: Some(peer),
    }
}

fn peer_failure_status(peer: &DaemonPeerSnapshot) -> Option<&'static str> {
    match peer.state.as_str() {
        "auth_failed" => Some("auth_failed"),
        "unreachable" => Some("unreachable"),
        "protocol_mismatch" => Some("protocol_mismatch"),
        _ => None,
    }
}

#[path = "peer_results_tests.rs"]
mod peer_results_tests;
