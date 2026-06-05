use std::time::Duration;

use super::super::{
    PeerEligibility, PeerManager, PeerPlacementRequest, PeerPlacementSelection, PeerSnapshot,
    PlacementFailure,
};
use super::support::{inventory, ping, record};

#[test]
fn connecting_peer_is_a_cold_fallback_not_warm() {
    let peers = manager();
    // No heartbeat has confirmed the peer yet: it is not warm (eligible), but it
    // remains placeable as a cold-dial fallback so a submit is never rejected
    // purely because warm-up has not finished.
    assert!(peers.eligible(&PeerEligibility::default()).is_empty());
    assert!(!peers.placeable(&PeerEligibility::default()).is_empty());
    assert_eq!(select(&peers).expect("cold fallback").node_id, "builder-a");
}

#[test]
fn connected_peer_is_warm_and_placeable() {
    let peers = manager();
    peers.mark_ping_success("builder-a", ping(), 1);
    assert!(!peers.eligible(&PeerEligibility::default()).is_empty());
    assert_eq!(select(&peers).expect("placeable").node_id, "builder-a");
}

#[tokio::test]
async fn wait_returns_immediately_when_no_peers_are_configured() {
    let peers = PeerManager::default();
    // A 30s budget would hang the test if the wait did not short-circuit.
    peers
        .wait_for_placeable_peer(&PeerEligibility::default(), Duration::from_secs(30))
        .await;
}

#[tokio::test]
async fn wait_resolves_once_a_peer_warms_up() {
    let peers = manager();
    let warmer = peers.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        warmer.mark_ping_success("builder-a", ping(), 1);
    });
    peers
        .wait_for_placeable_peer(&PeerEligibility::default(), Duration::from_secs(30))
        .await;
    assert!(!peers.eligible(&PeerEligibility::default()).is_empty());
}

fn manager() -> PeerManager {
    PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]))
}

fn select(peers: &PeerManager) -> Result<PeerSnapshot, PlacementFailure> {
    peers.select_placeable(PeerPlacementRequest {
        requirements: &PeerEligibility::default(),
        selection: PeerPlacementSelection::Sequential,
        task_run_id: "task-1",
        attempt: 1,
    })
}
