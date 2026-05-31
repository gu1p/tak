use super::super::PeerManager;
use super::support::{inventory, record};

#[test]
fn all_connection_targets_returns_every_configured_peer() {
    let peers = PeerManager::from_inventory(inventory(vec![
        record("builder-a", "tor", true, "tok-a"),
        record("builder-b", "tor", true, "tok-b"),
    ]));

    let mut targets = peers.all_connection_targets();
    targets.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].node_id, "builder-a");
    assert_eq!(targets[0].endpoint, "http://builder-a.onion");
    assert_eq!(targets[0].bearer_token, "tok-a");
    assert_eq!(targets[1].node_id, "builder-b");
    assert_eq!(targets[1].bearer_token, "tok-b");
}
