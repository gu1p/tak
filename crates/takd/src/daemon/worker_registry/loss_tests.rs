use tak_core::{remote_inventory::RemoteInventory, v2::RemoteRequirements};

use super::{
    WorkerRegistry,
    replacement_tests::{inventory, snapshot},
};

#[test]
fn first_observed_probe_failure_quarantines_until_valid_recovery() {
    let workers = registry();
    workers.mark_snapshot("worker-a", snapshot(8));
    let target = workers.probe_targets().pop().unwrap();

    workers.mark_probe_failure(&target);

    assert!(workers.scheduler_nodes().is_empty());
    assert!(workers.candidates(&requirements()).is_empty());
    assert!(workers.pending_node_losses().is_empty());

    assert!(workers.mark_probe_snapshot(&target, snapshot(8)));
    assert_eq!(workers.scheduler_nodes().len(), 1);
    assert_eq!(workers.candidates(&requirements()).len(), 1);
    let recovered = workers.probe_targets().pop().unwrap();
    workers.mark_probe_failure(&recovered);
    assert!(workers.pending_node_losses().is_empty());
}

#[test]
fn two_consecutive_observed_probe_failures_queue_exactly_one_loss() {
    let workers = registry();
    workers.mark_snapshot("worker-a", snapshot(8));
    let target = workers.probe_targets().pop().unwrap();

    workers.mark_probe_failure(&target);
    assert!(workers.pending_node_losses().is_empty());
    workers.mark_probe_failure(&target);
    assert_eq!(workers.pending_node_losses(), vec!["worker-a"]);
    workers.acknowledge_node_loss("worker-a");
    workers.mark_probe_failure(&target);
    assert!(workers.pending_node_losses().is_empty());
}

#[test]
fn unobserved_probe_failure_is_pending_until_the_worker_recovers() {
    let workers = registry();
    let target = workers.probe_targets().pop().unwrap();

    workers.mark_probe_failure(&target);

    assert!(workers.pending_node_losses().is_empty());
    assert_eq!(workers.pending_probe_failures(), vec!["worker-a"]);
    workers.mark_snapshot("worker-a", snapshot(8));
    assert!(workers.pending_probe_failures().is_empty());
}

#[test]
fn removing_an_observed_worker_from_inventory_is_node_loss() {
    let workers = registry();
    workers.mark_snapshot("worker-a", snapshot(8));
    remove_inventory(&workers);

    assert_eq!(workers.pending_node_losses(), vec!["worker-a"]);
}

#[test]
fn removing_a_quarantined_observed_worker_is_immediate_node_loss() {
    let workers = registry();
    workers.mark_snapshot("worker-a", snapshot(8));
    let target = workers.probe_targets().pop().unwrap();
    workers.mark_probe_failure(&target);

    remove_inventory(&workers);

    assert_eq!(workers.pending_node_losses(), vec!["worker-a"]);
}

fn registry() -> WorkerRegistry {
    let workers = WorkerRegistry::default();
    workers.apply_inventory(&inventory("http://127.0.0.1:9", "secret"), None);
    workers
}

fn requirements() -> RemoteRequirements {
    RemoteRequirements {
        pool: None,
        required_tags: vec![],
        required_capabilities: vec![],
        transport: None,
    }
}

fn remove_inventory(workers: &WorkerRegistry) {
    workers.apply_inventory(
        &RemoteInventory {
            version: 1,
            remotes: vec![],
        },
        None,
    );
}
