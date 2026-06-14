#![cfg(test)]
use super::{DaemonPeerSnapshot, peer_failure_status, result_from_peer, results_from_peers};

fn peer(node_id: &str, state: &str) -> DaemonPeerSnapshot {
    serde_json::from_str(&format!(
        r#"{{"node_id":"{node_id}","transport":"tor","endpoint":"http://{node_id}","state":"{state}"}}"#
    ))
    .expect("peer fixture")
}

#[test]
fn peer_failure_status_maps_known_failure_states() {
    assert_eq!(
        peer_failure_status(&peer("n", "auth_failed")),
        Some("auth_failed")
    );
    assert_eq!(
        peer_failure_status(&peer("n", "unreachable")),
        Some("unreachable")
    );
    assert_eq!(
        peer_failure_status(&peer("n", "protocol_mismatch")),
        Some("protocol_mismatch")
    );
    assert_eq!(peer_failure_status(&peer("n", "connected")), None);
}

#[test]
fn result_from_peer_copies_identity_and_marks_enabled() {
    let result = result_from_peer(peer("node-7", "connected"));
    assert_eq!(result.remote.node_id, "node-7");
    assert_eq!(result.remote.transport, "tor");
    assert!(result.remote.enabled);
    assert!(result.error.is_none());
    assert!(result.status.is_none());
}

#[test]
fn result_from_peer_surfaces_failure_state_as_error() {
    let result = result_from_peer(peer("node-7", "auth_failed"));
    assert_eq!(result.error.as_deref(), Some("auth_failed"));
}

fn direct_peer(node_id: &str) -> DaemonPeerSnapshot {
    serde_json::from_str(&format!(
        r#"{{"node_id":"{node_id}","transport":"direct","endpoint":"http://{node_id}","state":"connected"}}"#
    ))
    .expect("direct peer fixture")
}

#[test]
fn results_from_peers_drops_non_tor_and_sorts_remaining_by_node_id() {
    let peers = vec![
        peer("zeta", "connected"),
        direct_peer("a-direct"),
        peer("alpha", "connected"),
    ];
    let results = results_from_peers(peers, &[]);
    let ids: Vec<&str> = results.iter().map(|r| r.remote.node_id.as_str()).collect();
    assert_eq!(ids, vec!["alpha", "zeta"]);
}

#[test]
fn results_from_peers_keeps_only_node_filtered_tor_peers() {
    let peers = vec![peer("alpha", "connected"), peer("zeta", "connected")];
    let results = results_from_peers(peers, &["zeta".to_string()]);
    let ids: Vec<&str> = results.iter().map(|r| r.remote.node_id.as_str()).collect();
    assert_eq!(ids, vec!["zeta"]);
}
