use tak_proto::NodePingResponse;

use super::super::{PeerEligibility, PeerManager, PeerPlacementRequest, PeerPlacementSelection};
use super::support::{inventory, record};

#[test]
fn round_robin_placement_rotates_across_placeable_peers() {
    let manager = connected_manager(&[
        ("builder-a", 0, 0),
        ("builder-b", 0, 0),
        ("builder-c", 0, 0),
    ]);

    let selected = select_many(&manager, 5);

    assert_eq!(
        selected,
        [
            "builder-a",
            "builder-b",
            "builder-c",
            "builder-a",
            "builder-b"
        ]
    );
}

#[test]
fn round_robin_placement_skips_unplaceable_peers() {
    let manager = connected_manager(&[("builder-a", 0, 0), ("builder-b", 0, 0)]);
    manager.mark_auth_failed("builder-a", "auth rejected");

    let selected = select_many(&manager, 2);

    assert_eq!(selected, ["builder-b", "builder-b"]);
}

fn select_many(manager: &PeerManager, count: usize) -> Vec<String> {
    (0..count)
        .map(|index| {
            manager
                .select_placeable(PeerPlacementRequest {
                    requirements: &PeerEligibility::default(),
                    selection: PeerPlacementSelection::RoundRobin,
                    task_run_id: &format!("task-run-{index}"),
                    attempt: 1,
                })
                .expect("round-robin placement")
                .node_id
        })
        .collect()
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
