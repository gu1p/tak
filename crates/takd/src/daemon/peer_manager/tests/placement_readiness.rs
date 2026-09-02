use super::super::{PeerEligibility, PeerManager};
use super::support::{inventory, ping, record};

#[test]
fn connecting_peer_is_a_cold_fallback_not_warm() {
    let peers = manager();
    // No heartbeat has confirmed the peer yet: it is not warm (eligible), but it
    // remains placeable as a cold-dial fallback so a submit is never rejected
    // purely because warm-up has not finished.
    assert!(peers.eligible(&PeerEligibility::default()).is_empty());
    assert!(!peers.placeable(&PeerEligibility::default()).is_empty());
    assert_eq!(
        peers.placeable(&PeerEligibility::default())[0].node_id,
        "builder-a"
    );
}

#[test]
fn connected_peer_is_warm_and_placeable() {
    let peers = manager();
    peers.mark_ping_success("builder-a", ping(), 1);
    assert!(!peers.eligible(&PeerEligibility::default()).is_empty());
    assert_eq!(
        peers.placeable(&PeerEligibility::default())[0].node_id,
        "builder-a"
    );
}

fn manager() -> PeerManager {
    super::fixtures::peer_manager(inventory(vec![record("builder-a", "tor", true, "secret")]))
}
