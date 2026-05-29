use tak_proto::NodePingResponse;

use super::super::{PeerEligibility, PeerManager, PeerPlacementRequest, PeerPlacementSelection};
use super::support::{inventory, record};

#[test]
fn sequential_placement_preserves_first_placeable_peer() {
    let manager = connected_manager(&[("builder-a", 4, 0), ("builder-b", 0, 0)]);

    let selected = manager
        .select_placeable(PeerPlacementRequest {
            requirements: &PeerEligibility::default(),
            selection: PeerPlacementSelection::Sequential,
            task_run_id: "task-run-1",
            attempt: 1,
        })
        .expect("sequential placement");

    assert_eq!(selected.node_id, "builder-a");
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

fn connected_manager(nodes: &[(&str, u32, u32)]) -> PeerManager {
    let manager = PeerManager::from_inventory(inventory(
        nodes
            .iter()
            .map(|(node_id, _, _)| record(node_id, "tor", true, "secret"))
            .collect(),
    ));
    for (node_id, active_jobs, queue_depth) in nodes {
        manager.mark_ping_success(node_id, ping(node_id, *active_jobs, *queue_depth), 1);
    }
    manager
}

fn ping(node_id: &str, active_jobs: u32, queue_depth: u32) -> NodePingResponse {
    NodePingResponse {
        node_id: node_id.to_string(),
        protocol_version: "v1".to_string(),
        health: "healthy".to_string(),
        active_job_count: active_jobs,
        queue_depth,
        resource_summary: "cpu_available=8.00 memory_available_mb=16384".to_string(),
    }
}
