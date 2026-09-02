use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::worker_v2::{WorkerResources, WorkerSnapshot};

use super::WorkerRegistry;

#[test]
fn stale_probe_success_cannot_replace_a_new_connection_snapshot() {
    let registry = WorkerRegistry::default();
    registry.apply_inventory(&inventory("http://127.0.0.1:1", "old"), None);
    let stale = registry.probe_targets().pop().unwrap();
    replace_a_to_b_to_a(&registry);
    registry.mark_snapshot("worker-a", snapshot(9));

    registry.mark_probe_snapshot(&stale, snapshot(1));

    assert_eq!(registry.scheduler_nodes()[0].execution_capacity, 9);
}

#[test]
fn stale_probe_failure_cannot_clear_a_new_connection_snapshot() {
    let registry = WorkerRegistry::default();
    registry.apply_inventory(&inventory("http://127.0.0.1:1", "old"), None);
    let stale = registry.probe_targets().pop().unwrap();
    replace_a_to_b_to_a(&registry);
    registry.mark_snapshot("worker-a", snapshot(9));

    registry.mark_probe_failure(&stale);

    assert_eq!(registry.scheduler_nodes()[0].execution_capacity, 9);
}

#[test]
fn probe_loss_confirmation_cannot_cross_connection_generations() {
    let registry = WorkerRegistry::default();
    registry.apply_inventory(&inventory("http://127.0.0.1:1", "old"), None);
    registry.mark_snapshot("worker-a", snapshot(9));
    let stale = registry.probe_targets().pop().unwrap();
    registry.mark_probe_failure(&stale);
    replace_a_to_b_to_a(&registry);
    registry.mark_snapshot("worker-a", snapshot(9));
    let current = registry.probe_targets().pop().unwrap();

    registry.mark_probe_failure(&current);
    registry.mark_probe_failure(&stale);

    assert!(registry.scheduler_nodes().is_empty());
    assert!(registry.pending_node_losses().is_empty());
}

fn replace_a_to_b_to_a(registry: &WorkerRegistry) {
    registry.apply_inventory(&inventory("http://127.0.0.1:2", "middle"), None);
    registry.apply_inventory(&inventory("http://127.0.0.1:1", "old"), None);
}

pub(super) fn inventory(endpoint: &str, token: &str) -> RemoteInventory {
    RemoteInventory {
        version: 1,
        remotes: vec![RemoteRecord {
            node_id: "worker-a".into(),
            display_name: "worker-a".into(),
            base_url: endpoint.into(),
            bearer_token: token.into(),
            pools: vec![],
            tags: vec![],
            capabilities: vec![],
            transport: "direct".into(),
            enabled: true,
        }],
    }
}

pub(super) fn snapshot(slots: u32) -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "worker-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: WorkerResources {
            cpu_millis: 1,
            memory_bytes: 1,
            execution_slots: slots,
        },
        usage: WorkerResources {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: 0,
        },
        queue_depth: 0,
        cached_content: vec![],
        processes: vec![],
    }
}
