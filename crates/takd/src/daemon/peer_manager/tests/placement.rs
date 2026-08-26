use super::super::{PeerEligibility, PeerPlacementRequest, PeerPlacementSelection};

#[path = "placement/fixtures.rs"]
mod placement_fixtures;

use placement_fixtures::connected_manager;

#[test]
fn sequential_placement_preserves_first_placeable_peer() {
    let manager = connected_manager(&[("builder-a", 4, 0), ("builder-b", 0, 0)]);

    let selected = manager
        .select_placeable(PeerPlacementRequest {
            requirements: &PeerEligibility::default(),
            selection: PeerPlacementSelection::Sequential,
            task_run_id: "task-run-1",
            attempt: 1,
            excluded_node_ids: &[],
        })
        .expect("sequential placement");

    assert_eq!(selected.node_id, "builder-a");
}

#[test]
fn sequential_placement_skips_failed_nodes() {
    let manager = connected_manager(&[("builder-a", 0, 0), ("builder-b", 0, 0)]);
    let excluded = vec!["builder-a".to_string()];

    let selected = manager
        .select_placeable(PeerPlacementRequest {
            requirements: &PeerEligibility::default(),
            selection: PeerPlacementSelection::Sequential,
            task_run_id: "task-run-1",
            attempt: 1,
            excluded_node_ids: &excluded,
        })
        .expect("replacement placement");

    assert_eq!(selected.node_id, "builder-b");
}

#[test]
fn shuffle_placement_prefers_less_loaded_fitting_peer() {
    let manager = connected_manager(&[("builder-a", 4, 0), ("builder-b", 0, 0)]);

    let selected = manager
        .select_placeable(PeerPlacementRequest {
            requirements: &PeerEligibility::default(),
            selection: PeerPlacementSelection::Shuffle,
            task_run_id: "task-run-1",
            attempt: 1,
            excluded_node_ids: &[],
        })
        .expect("shuffle placement");

    assert_eq!(selected.node_id, "builder-b");
}

#[test]
fn shuffle_placement_spreads_equal_peers_across_assignments() {
    let manager = connected_manager(&[("builder-a", 0, 0), ("builder-b", 0, 0)]);

    let selected = (0..4)
        .map(|index| {
            manager
                .select_placeable(PeerPlacementRequest {
                    requirements: &PeerEligibility::default(),
                    selection: PeerPlacementSelection::Shuffle,
                    task_run_id: &format!("task-run-{index}"),
                    attempt: 1,
                    excluded_node_ids: &[],
                })
                .expect("shuffle placement")
                .node_id
        })
        .collect::<Vec<_>>();

    assert_eq!(
        selected.iter().filter(|node| *node == "builder-a").count(),
        2
    );
    assert_eq!(
        selected.iter().filter(|node| *node == "builder-b").count(),
        2
    );
}
