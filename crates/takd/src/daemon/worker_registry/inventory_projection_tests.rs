use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};

use super::WorkerRegistry;
use crate::daemon::scheduler::SchedulerNode;

#[test]
fn scheduler_nodes_expose_the_latest_live_inventory_requirements() {
    let registry = WorkerRegistry::default();
    registry.apply_inventory(&inventory(true), None);
    registry.mark_snapshot("worker-a", snapshot());
    assert_inventory(&registry.scheduler_nodes()[0], true);

    registry.apply_inventory(&inventory(false), None);
    let nodes = registry.scheduler_nodes();
    assert_eq!(nodes.len(), 1, "same healthy worker remains schedulable");
    assert_inventory(&nodes[0], false);
}

fn assert_inventory(node: &SchedulerNode, present: bool) {
    assert_eq!(node.pools.iter().any(|value| value == "build"), present);
    assert_eq!(node.tags.iter().any(|value| value == "builder"), present);
    assert_eq!(
        node.capabilities.iter().any(|value| value == "linux"),
        present
    );
}

fn inventory(eligible: bool) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "worker-a".into(),
            display_name: "worker-a".into(),
            base_url: "http://127.0.0.1:9".into(),
            bearer_token: "secret".into(),
            pools: values(eligible, "build"),
            tags: values(eligible, "builder"),
            capabilities: values(eligible, "linux"),
            transport: "direct".into(),
            enabled: true,
        }],
    }
}

fn values(present: bool, value: &str) -> Vec<String> {
    if present { vec![value.into()] } else { vec![] }
}

fn snapshot() -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "worker-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(1),
        usage: resources(0),
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}

fn resources(execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 8_000,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        execution_slots,
    }
}
