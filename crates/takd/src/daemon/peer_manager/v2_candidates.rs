use tak_core::v2::{PlacementCandidate, RemoteRequirements};
use tak_proto::worker_v2::WorkerSnapshot;

use super::PeerManager;
use crate::daemon::scheduler::SchedulerNode;

impl PeerManager {
    pub fn remote_candidates(&self, requirements: &RemoteRequirements) -> Vec<PlacementCandidate> {
        self.workers.candidates(requirements)
    }

    pub fn mark_worker_snapshot(&self, expected_node_id: &str, snapshot: WorkerSnapshot) {
        self.workers.mark_snapshot(expected_node_id, snapshot);
    }

    pub fn worker_target(
        &self,
        node_id: &str,
        expected_transport: &str,
    ) -> Option<crate::daemon::worker_registry::WorkerConnectionTarget> {
        self.workers.target(node_id, expected_transport)
    }

    pub fn scheduler_nodes(&self) -> Vec<SchedulerNode> {
        self.workers.scheduler_nodes()
    }

    pub(crate) fn pending_worker_node_losses(&self) -> Vec<String> {
        self.workers.pending_node_losses()
    }

    pub(crate) fn acknowledge_worker_node_loss(&self, node_id: &str) {
        self.workers.acknowledge_node_loss(node_id);
    }

    pub(crate) fn pending_worker_probe_failures(&self) -> Vec<String> {
        self.workers.pending_probe_failures()
    }

    pub(crate) fn acknowledge_worker_probe_failure(&self, node_id: &str) {
        self.workers.acknowledge_probe_failure(node_id);
    }
}
