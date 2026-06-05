use super::super::{PeerEligibility, PeerManager, PeerPlacementRequest, PeerPlacementSelection};
use super::support::{inventory, record};

#[test]
fn unreachable_peers_are_retryable_placement_failures() {
    let manager = manager_with_peer("builder-a");
    manager.mark_ping_failure("builder-a", "dial timed out");
    manager.mark_ping_failure("builder-a", "dial timed out");

    let failure = select_failure(&manager, PeerEligibility::default());

    assert_eq!(failure.code(), "all_tor_peers_unreachable");
    assert!(failure.is_retryable());
}

#[test]
fn missing_inventory_match_is_not_retryable() {
    let failure = select_failure(
        &manager_with_peer("builder-a"),
        PeerEligibility {
            pool: Some("release".into()),
            ..PeerEligibility::default()
        },
    );

    assert_eq!(failure.code(), "no_matching_tor_peers");
    assert!(!failure.is_retryable());
}

#[test]
fn impossible_capacity_is_not_retryable() {
    let manager = manager_with_peer("builder-a");
    let mut ping = super::support::ping();
    ping.resource_summary = "cpu_available=8.00 cpu_total=8.00".into();
    manager.mark_ping_success("builder-a", ping, 1);

    let failure = select_failure(
        &manager,
        PeerEligibility {
            cpu_cores: Some(16.0),
            ..PeerEligibility::default()
        },
    );

    assert_eq!(
        failure.code(),
        "resource_requirements_exceed_worker_capacity"
    );
    assert!(!failure.is_retryable());
}

#[test]
fn no_currently_placeable_peer_is_retryable() {
    let manager = manager_with_peer("builder-a");
    let mut ping = super::support::ping();
    ping.health = "recovering".into();
    manager.mark_ping_success("builder-a", ping, 1);

    let failure = select_failure(&manager, PeerEligibility::default());

    assert_eq!(failure.code(), "no_placeable_tor_peers");
    assert!(failure.is_retryable());
}

fn manager_with_peer(node_id: &str) -> PeerManager {
    PeerManager::from_inventory(inventory(vec![record(node_id, "tor", true, "secret")]))
}

fn select_failure(
    manager: &PeerManager,
    requirements: PeerEligibility,
) -> super::super::PlacementFailure {
    manager
        .select_placeable(PeerPlacementRequest {
            requirements: &requirements,
            selection: PeerPlacementSelection::Sequential,
            task_run_id: "task-run-1",
            attempt: 1,
        })
        .expect_err("placement should fail")
}
