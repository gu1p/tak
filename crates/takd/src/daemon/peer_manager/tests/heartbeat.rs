use super::super::heartbeat::unix_epoch_ms;
use super::super::{PeerEligibility, PeerManager, PeerState};
use super::support::{inventory, ping, record};

#[test]
fn heartbeat_failures_move_connected_to_degraded_then_unreachable() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    manager.mark_ping_success("builder-a", ping(), 12);

    manager.mark_ping_failure("builder-a", "timeout");
    assert_eq!(manager.snapshots()[0].state, PeerState::Degraded);

    manager.mark_ping_failure("builder-a", "timeout");
    assert_eq!(manager.snapshots()[0].state, PeerState::Unreachable);
}

#[test]
fn heartbeat_rejects_ping_from_unexpected_node_id() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.node_id = "builder-b".to_string();

    manager.mark_ping_success("builder-a", ping, 12);

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::ProtocolMismatch);
    assert!(
        snapshot
            .last_error_summary
            .unwrap()
            .contains("unexpected node_id")
    );
}

#[test]
fn heartbeat_rejects_ping_with_wrong_protocol_version() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.protocol_version = "v0".to_string();

    manager.mark_ping_success("builder-a", ping, 12);

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::ProtocolMismatch);
    assert!(snapshot.last_error_summary.unwrap().contains("protocol"));
}

#[test]
fn unhealthy_ping_is_visible_but_not_placeable() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.health = "recovering".to_string();

    manager.mark_ping_success("builder-a", ping, 12);

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::Degraded);
    assert!(manager.placeable(&PeerEligibility::default()).is_empty());
    assert!(snapshot.last_error_summary.unwrap().contains("recovering"));
}

#[test]
fn heartbeat_failures_back_off_before_next_retry() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let now = unix_epoch_ms();

    assert_eq!(manager.heartbeat_targets_due(now).len(), 1);

    manager.mark_ping_failure("builder-a", "timeout");

    assert!(manager.heartbeat_targets_due(now).is_empty());
    assert_eq!(manager.heartbeat_targets_due(now + 61_000).len(), 1);
}
