use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_core::v2::RemoteRequirements;
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};
use takd::PeerManager;

#[test]
fn placement_candidates_preserve_authored_inventory_order() {
    let peers = PeerManager::default();
    peers.apply_inventory(RemoteInventory {
        version: 1,
        remotes: vec![record("worker-z"), record("worker-a")],
    });
    for node in ["worker-z", "worker-a"] {
        peers.mark_worker_snapshot(node, snapshot(node));
    }

    let candidates = peers.remote_candidates(&RemoteRequirements {
        pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport: None,
    });
    let nodes = candidates
        .iter()
        .map(|candidate| candidate.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(nodes, ["worker-z", "worker-a"]);
}

fn record(node_id: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: format!("http://127.0.0.1/{node_id}"),
        bearer_token: "secret".into(),
        pools: Vec::new(),
        tags: Vec::new(),
        capabilities: Vec::new(),
        transport: "direct".into(),
        enabled: true,
    }
}

fn snapshot(node_id: &str) -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: node_id.into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources {
            cpu_millis: 1_000,
            memory_bytes: 1_000,
            execution_slots: 1,
        },
        usage: WorkerResources {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: 0,
        },
        queue_depth: 0,
        cached_content: Vec::new(),
        processes: Vec::new(),
    }
}
