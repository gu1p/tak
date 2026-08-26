use tak_proto::NodePingResponse;

use super::super::super::PeerManager;
use super::super::support::{inventory, record};

pub(super) fn connected_manager(nodes: &[(&str, u32, u32)]) -> PeerManager {
    let manager = super::super::fixtures::peer_manager(inventory(
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
