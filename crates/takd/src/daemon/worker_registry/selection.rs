use std::time::Duration;

use tak_core::v2::{PlacementCandidate, PlacementKind, RemoteRequirements};
use tak_proto::worker_v2::PROTOCOL_VERSION;

use super::{WorkerConnectionTarget, WorkerEntry, WorkerRegistry};
use crate::daemon::scheduler::SchedulerNode;

const SNAPSHOT_FRESHNESS: Duration = Duration::from_secs(60);

impl WorkerRegistry {
    pub(crate) fn candidates(&self, requirements: &RemoteRequirements) -> Vec<PlacementCandidate> {
        let entries = self.inner.lock().expect("worker registry lock poisoned");
        let mut candidates = entries
            .values()
            .filter(|entry| usable(entry) && matches_requirements(entry, requirements))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|entry| entry.inventory_order);
        candidates
            .into_iter()
            .map(|entry| PlacementCandidate {
                node_id: entry.remote.node_id.clone(),
                kind: PlacementKind::Remote,
                transport: Some(entry.remote.transport.clone()),
                reason: "healthy protocol-v2 worker matches authored requirements".into(),
                tier: 0,
                requirements: Some(requirements.clone()),
            })
            .collect()
    }

    pub(crate) fn target(
        &self,
        node_id: &str,
        expected_transport: &str,
    ) -> Option<WorkerConnectionTarget> {
        self.inner
            .lock()
            .expect("worker registry lock poisoned")
            .get(node_id)
            .filter(|entry| entry.remote.transport == expected_transport)
            .map(connection_target)
    }

    pub(crate) fn scheduler_nodes(&self) -> Vec<SchedulerNode> {
        self.inner
            .lock()
            .expect("worker registry lock poisoned")
            .values()
            .filter(|entry| usable(entry))
            .filter_map(scheduler_node)
            .collect()
    }
}

pub(super) fn connection_target(entry: &WorkerEntry) -> WorkerConnectionTarget {
    WorkerConnectionTarget {
        node_id: entry.remote.node_id.clone(),
        endpoint: entry.remote.base_url.clone(),
        bearer_token: entry.remote.bearer_token.clone(),
        transport: entry.remote.transport.clone(),
    }
}

fn scheduler_node(entry: &WorkerEntry) -> Option<SchedulerNode> {
    let snapshot = &entry.snapshot.as_ref()?.value;
    Some(SchedulerNode {
        node_id: entry.remote.node_id.clone(),
        transport: Some(entry.remote.transport.clone()),
        pools: entry.remote.pools.iter().cloned().collect(),
        tags: entry.remote.tags.iter().cloned().collect(),
        capabilities: entry.remote.capabilities.iter().cloned().collect(),
        cpu_capacity_millis: snapshot.capacity.cpu_millis,
        cpu_used_millis: snapshot.usage.cpu_millis,
        memory_capacity_bytes: snapshot.capacity.memory_bytes,
        memory_used_bytes: snapshot.usage.memory_bytes,
        execution_capacity: snapshot.capacity.execution_slots,
        execution_used: snapshot.usage.execution_slots,
        queue_depth: snapshot.queue_depth,
        cached_content: snapshot.cached_content.iter().cloned().collect(),
        processes: snapshot.processes.clone(),
    })
}

fn usable(entry: &WorkerEntry) -> bool {
    entry.snapshot.as_ref().is_some_and(|snapshot| {
        snapshot.received_at.elapsed() <= SNAPSHOT_FRESHNESS
            && snapshot.value.protocol_version == PROTOCOL_VERSION
            && snapshot.value.healthy
    })
}

fn matches_requirements(entry: &WorkerEntry, requirements: &RemoteRequirements) -> bool {
    let remote = &entry.remote;
    requirements
        .transport
        .as_ref()
        .is_none_or(|value| value == &remote.transport)
        && requirements
            .pool
            .as_ref()
            .is_none_or(|value| remote.pools.contains(value))
        && requirements
            .required_tags
            .iter()
            .all(|value| remote.tags.contains(value))
        && requirements.required_capabilities.iter().all(|value| {
            remote.capabilities.contains(value)
                || value
                    .strip_prefix("node:")
                    .is_some_and(|selector| node_matches(selector, remote))
        })
}

fn node_matches(selector: &str, remote: &tak_core::remote_inventory::RemoteRecord) -> bool {
    !selector.is_empty()
        && (selector == remote.node_id
            || selector == remote.display_name
            || remote.node_id.starts_with(selector)
            || selector == tak_core::remote_alias_for_node_id(&remote.node_id))
}
