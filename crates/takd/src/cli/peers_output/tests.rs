use super::*;

#[test]
fn takd_peers_renders_empty_and_all_peer_states() {
    assert!(
        render_peers(&[]).contains("no remote workers configured"),
        "empty peers output should describe the empty state"
    );

    let output = render_peers(&[
        peer("builder-connecting", "connecting"),
        peer("builder-connected", "connected"),
        peer("builder-degraded", "degraded"),
        peer("builder-unreachable", "unreachable"),
        peer("builder-protocol", "protocol_mismatch"),
        peer("builder-auth", "auth_failed"),
        peer("builder-disconnected", "disconnected"),
        peer_with_transport("builder-direct", "direct", "ready"),
    ]);

    for expected in [
        "connecting",
        "connected",
        "degraded",
        "unreachable",
        "protocol_mismatch",
        "auth_failed",
        "disconnected",
        "builder-direct direct",
    ] {
        assert!(output.contains(expected), "missing {expected}:\n{output}");
    }
}

fn peer(node_id: &str, state: &str) -> PeerRow {
    peer_with_transport(node_id, "tor", state)
}

fn peer_with_transport(node_id: &str, transport: &str, state: &str) -> PeerRow {
    PeerRow {
        node_id: node_id.to_string(),
        transport: transport.to_string(),
        state: state.to_string(),
        last_heartbeat_ms: None,
        active_job_count: None,
        queue_depth: None,
    }
}
