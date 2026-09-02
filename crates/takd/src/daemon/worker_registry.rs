use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tak_core::remote_inventory::{RemoteInventory, RemoteRecord};
use tak_proto::worker_v2::{PROTOCOL_VERSION, WorkerSnapshot};

use super::peer_manager::LocalNodeIdentity;

#[cfg(test)]
mod inventory_projection_tests;
mod loss;
#[cfg(test)]
mod loss_tests;
mod probe;
#[cfg(test)]
mod replacement_tests;
mod selection;

#[derive(Clone, PartialEq, Eq)]
pub struct WorkerConnectionTarget {
    pub node_id: String,
    pub endpoint: String,
    pub bearer_token: String,
    pub transport: String,
}

impl fmt::Debug for WorkerConnectionTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkerConnectionTarget")
            .field("node_id", &self.node_id)
            .field("endpoint", &self.endpoint)
            .field("bearer_token", &"[redacted]")
            .field("transport", &self.transport)
            .finish()
    }
}

#[derive(Clone, Default)]
pub(crate) struct WorkerRegistry {
    inner: Arc<Mutex<BTreeMap<String, WorkerEntry>>>,
    pending_node_losses: Arc<Mutex<BTreeSet<String>>>,
    pending_probe_failures: Arc<Mutex<BTreeSet<String>>>,
    next_connection_generation: Arc<AtomicU64>,
}

#[derive(Clone)]
struct WorkerEntry {
    remote: RemoteRecord,
    snapshot: Option<ObservedSnapshot>,
    probe_failure: loss::ProbeFailureConfirmation,
    connection_generation: u64,
    inventory_order: usize,
}

#[derive(Clone)]
struct ObservedSnapshot {
    value: WorkerSnapshot,
    received_at: Instant,
}

#[derive(Clone)]
struct WorkerProbeTarget {
    connection: WorkerConnectionTarget,
    generation: u64,
}

impl WorkerRegistry {
    pub(crate) fn apply_inventory(
        &self,
        inventory: &RemoteInventory,
        local_identity: Option<&LocalNodeIdentity>,
    ) {
        let next = inventory
            .enabled_remotes()
            .filter(|remote| {
                local_identity
                    .is_none_or(|local| !local.matches_peer(&remote.node_id, &remote.base_url))
            })
            .enumerate()
            .map(|(order, remote)| (remote.node_id.clone(), (order, remote.clone())))
            .collect::<BTreeMap<_, _>>();
        let mut entries = self.inner.lock().expect("worker registry lock poisoned");
        let removed = entries
            .iter()
            .filter(|(node_id, entry)| {
                !next.contains_key(*node_id) && entry.removal_confirms_loss()
            })
            .map(|(node_id, _)| node_id.clone())
            .collect::<Vec<_>>();
        entries.retain(|node_id, _| next.contains_key(node_id));
        for (node_id, (inventory_order, remote)) in next {
            match entries.get_mut(&node_id) {
                Some(entry) if same_connection(&entry.remote, &remote) => {
                    entry.remote = remote;
                    entry.inventory_order = inventory_order;
                }
                _ => {
                    let connection_generation = self
                        .next_connection_generation
                        .fetch_add(1, Ordering::Relaxed)
                        .saturating_add(1);
                    entries.insert(
                        node_id,
                        WorkerEntry {
                            remote,
                            snapshot: None,
                            probe_failure: loss::ProbeFailureConfirmation::Clear,
                            connection_generation,
                            inventory_order,
                        },
                    );
                }
            }
        }
        drop(entries);
        self.pending_node_losses
            .lock()
            .expect("worker loss lock poisoned")
            .extend(removed);
    }

    pub(crate) fn mark_snapshot(&self, expected_node_id: &str, snapshot: WorkerSnapshot) {
        let mut entries = self.inner.lock().expect("worker registry lock poisoned");
        let Some(entry) = entries.get_mut(expected_node_id) else {
            return;
        };
        let valid =
            snapshot.protocol_version == PROTOCOL_VERSION && snapshot.node_id == expected_node_id;
        let lost = entry.snapshot.is_some() && !valid;
        entry.snapshot = valid.then_some(ObservedSnapshot {
            value: snapshot,
            received_at: Instant::now(),
        });
        if valid {
            entry.probe_failure = loss::ProbeFailureConfirmation::Clear;
        } else if lost {
            entry.probe_failure = loss::ProbeFailureConfirmation::Confirmed;
        }
        drop(entries);
        if valid {
            self.acknowledge_probe_failure(expected_node_id);
        } else if lost {
            self.queue_node_loss(expected_node_id);
        }
    }

    fn probe_targets(&self) -> Vec<WorkerProbeTarget> {
        self.inner
            .lock()
            .expect("worker registry lock poisoned")
            .values()
            .map(|entry| WorkerProbeTarget {
                connection: selection::connection_target(entry),
                generation: entry.connection_generation,
            })
            .collect()
    }

    fn mark_probe_snapshot(&self, target: &WorkerProbeTarget, snapshot: WorkerSnapshot) -> bool {
        let mut entries = self.inner.lock().expect("worker registry lock poisoned");
        let Some(entry) = entries.get_mut(&target.connection.node_id) else {
            return false;
        };
        if entry.connection_generation != target.generation {
            return false;
        }
        let valid = snapshot.protocol_version == PROTOCOL_VERSION
            && snapshot.node_id == target.connection.node_id;
        let lost = !valid
            && (entry.snapshot.is_some()
                || entry.probe_failure == loss::ProbeFailureConfirmation::Awaiting);
        entry.snapshot = valid.then_some(ObservedSnapshot {
            value: snapshot,
            received_at: Instant::now(),
        });
        if valid {
            entry.probe_failure = loss::ProbeFailureConfirmation::Clear;
        } else if lost {
            entry.probe_failure = loss::ProbeFailureConfirmation::Confirmed;
        }
        drop(entries);
        if valid {
            self.acknowledge_probe_failure(&target.connection.node_id);
        } else if lost {
            self.queue_node_loss(&target.connection.node_id);
        } else {
            self.queue_probe_failure(&target.connection.node_id);
        }
        valid
    }
}

fn same_connection(left: &RemoteRecord, right: &RemoteRecord) -> bool {
    left.base_url == right.base_url
        && left.bearer_token == right.bearer_token
        && left.transport == right.transport
}
