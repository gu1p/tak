use std::collections::BTreeMap;

use tak_core::v2::RemoteSelection;

use self::support::{committed, counts, node};

#[path = "v2_scheduler_first_available_behavior/support.rs"]
mod support;

#[test]
fn balanced_primary_tier_places_ten_equal_jobs_five_per_remote() {
    let (_temp, store, run_id) = committed("first-balanced", 10, RemoteSelection::Balanced);
    let nodes = [
        node("worker-a", 10),
        node("worker-b", 10),
        node("local", 10),
    ];
    for _ in 0..10 {
        store.reserve_next(&nodes).unwrap().unwrap();
    }

    assert_eq!(
        counts(&store, &run_id),
        BTreeMap::from([("worker-a".into(), 5), ("worker-b".into(), 5)])
    );
}

#[test]
fn local_tier_is_used_only_when_no_primary_remote_can_reserve() {
    let (_temp, store, _run_id) = committed("first-fallback", 1, RemoteSelection::Balanced);
    let unavailable_or_full = [
        node("worker-a", 1).with_execution_usage(1),
        node("local", 1),
    ];

    let dispatch = store.reserve_next(&unavailable_or_full).unwrap().unwrap();
    assert_eq!(dispatch.node_id, "local");
}

#[test]
fn round_robin_cursor_never_reaches_fallback_while_primary_can_reserve() {
    let (_temp, store, _run_id) = committed("first-round-robin", 3, RemoteSelection::RoundRobin);
    let nodes = [
        node("worker-a", 10),
        node("worker-b", 10),
        node("local", 10),
    ];

    let assigned = (0..3)
        .map(|_| store.reserve_next(&nodes).unwrap().unwrap().node_id)
        .collect::<Vec<_>>();
    assert_eq!(assigned, ["worker-a", "worker-b", "worker-a"]);
}

#[test]
fn round_robin_primary_cursor_survives_a_fallback_reservation() {
    let (_temp, store, _run_id) =
        committed("first-round-robin-recovery", 3, RemoteSelection::RoundRobin);
    let primary = [
        node("worker-a", 10),
        node("worker-b", 10),
        node("local", 10),
    ];
    assert_eq!(
        store.reserve_next(&primary).unwrap().unwrap().node_id,
        "worker-a"
    );

    let fallback_only = [node("local", 10)];
    assert_eq!(
        store.reserve_next(&fallback_only).unwrap().unwrap().node_id,
        "local"
    );
    assert_eq!(
        store.reserve_next(&primary).unwrap().unwrap().node_id,
        "worker-b"
    );
}
