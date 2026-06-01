use super::super::heartbeat::unix_epoch_ms;
use super::super::{PeerEligibility, PeerManager, PeerState};
use super::support::{inventory, ping, record};

#[test]
fn unhealthy_ping_records_successful_connection_without_reconnect_backoff() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.health = "recovering".to_string();
    let now = unix_epoch_ms();

    manager.mark_ping_success("builder-a", ping, 12);

    let snapshot = manager.snapshots().pop().expect("snapshot");
    assert_eq!(snapshot.state, PeerState::Degraded);
    assert_eq!(snapshot.reconnect_attempts, 0);
    assert!(snapshot.last_heartbeat_ms.is_some());
    assert!(snapshot.last_successful_connection_ms.is_some());
    assert!(snapshot.last_error_summary.unwrap().contains("recovering"));
    assert!(manager.placeable(&PeerEligibility::default()).is_empty());
    assert!(manager.heartbeat_targets_due(now + 5_000).is_empty());
    assert_eq!(manager.heartbeat_targets_due(now + 16_000).len(), 1);
}
