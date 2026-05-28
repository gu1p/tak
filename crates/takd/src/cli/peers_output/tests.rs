use super::*;
use takd::{PeerSnapshot, PeerState};

#[test]
fn takd_peers_renders_empty_and_all_peer_states() {
    assert!(
        render_peers(&[]).contains("no tor peers configured"),
        "empty peers output should describe the empty state"
    );

    let output = render_peers(&[
        peer("builder-connecting", PeerState::Connecting),
        peer("builder-connected", PeerState::Connected),
        peer("builder-degraded", PeerState::Degraded),
        peer("builder-unreachable", PeerState::Unreachable),
        peer("builder-protocol", PeerState::ProtocolMismatch),
        peer("builder-auth", PeerState::AuthFailed),
        peer("builder-disconnected", PeerState::Disconnected),
    ]);

    for expected in [
        "connecting",
        "connected",
        "degraded",
        "unreachable",
        "protocol_mismatch",
        "auth_failed",
        "disconnected",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
}

fn peer(node_id: &str, state: PeerState) -> PeerSnapshot {
    PeerSnapshot {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        transport: "tor".to_string(),
        endpoint: format!("http://{node_id}.onion"),
        state,
        last_heartbeat_ms: None,
        last_successful_connection_ms: None,
        last_error_summary: None,
        active_job_count: None,
        queue_depth: None,
        resource_summary: None,
        protocol_version: None,
        heartbeat_rtt_ms: None,
        reconnect_attempts: 0,
        pools: Vec::new(),
        tags: Vec::new(),
        capabilities: Vec::new(),
    }
}
