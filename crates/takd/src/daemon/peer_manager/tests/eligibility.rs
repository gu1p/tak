use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::NodePingResponse;

use super::super::{PeerEligibility, PeerManager};

#[test]
fn busy_but_feasible_peer_remains_placeable_so_worker_queue_can_admit() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.resource_summary =
        "cpu_available=0.00 cpu_total=8.00 memory_available_mb=0 memory_total_mb=16384".to_string();
    manager.mark_ping_success("builder-a", ping, 12);

    let requirements = PeerEligibility {
        cpu_cores: Some(4.0),
        memory_mb: Some(4096),
        ..PeerEligibility::default()
    };

    let placeable = manager.placeable(&requirements);
    assert_eq!(placeable.len(), 1);
    assert_eq!(placeable[0].node_id, "builder-a");
}

#[test]
fn requirements_above_known_total_capacity_are_not_placeable() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    let mut ping = ping();
    ping.resource_summary =
        "cpu_available=8.00 cpu_total=8.00 memory_available_mb=16384 memory_total_mb=16384"
            .to_string();
    manager.mark_ping_success("builder-a", ping, 12);

    let requirements = PeerEligibility {
        cpu_cores: Some(16.0),
        memory_mb: Some(4096),
        ..PeerEligibility::default()
    };

    assert!(manager.placeable(&requirements).is_empty());
    let err = super::super::first_placeable_or_error(&manager.snapshots(), &requirements)
        .expect_err("oversized task should explain placement failure");
    let rendered = format!("{err:#}");
    assert!(rendered.contains("No known remote worker satisfies this task's requirements"));
    assert!(rendered.contains("cpu: 16.00"));
    assert!(rendered.contains("largest known worker"));
    assert!(rendered.contains("cpu: 8.00"));
    assert!(rendered.contains("requirements are reduced"));
}

#[test]
fn placement_diagnostics_distinguish_auth_failed_peers() {
    let manager =
        PeerManager::from_inventory(inventory(vec![record("builder-a", "tor", true, "secret")]));
    manager.mark_auth_failed("builder-a", "auth rejected");

    let err =
        super::super::first_placeable_or_error(&manager.snapshots(), &PeerEligibility::default())
            .expect_err("auth failed peer should not be placeable");

    assert!(format!("{err:#}").contains("auth failed"));
}

fn inventory(remotes: Vec<RemoteRecord>) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes,
    }
}

fn record(node_id: &str, transport: &str, enabled: bool, bearer_token: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.onion"),
        bearer_token: bearer_token.to_string(),
        pools: vec!["build".to_string()],
        tags: vec!["linux".to_string()],
        capabilities: vec!["docker".to_string()],
        transport: transport.to_string(),
        enabled,
    }
}

fn ping() -> NodePingResponse {
    NodePingResponse {
        node_id: "builder-a".to_string(),
        protocol_version: "v1".to_string(),
        health: "healthy".to_string(),
        active_job_count: 1,
        queue_depth: 0,
        resource_summary: "cpu_available=4".to_string(),
    }
}
