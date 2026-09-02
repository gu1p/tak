use super::{WorkerEntry, WorkerProbeTarget, WorkerRegistry};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum ProbeFailureConfirmation {
    #[default]
    Clear,
    Awaiting,
    Confirmed,
}

impl WorkerEntry {
    pub(super) fn removal_confirms_loss(&self) -> bool {
        self.snapshot.is_some() || self.probe_failure == ProbeFailureConfirmation::Awaiting
    }
}

impl WorkerRegistry {
    pub(crate) fn pending_node_losses(&self) -> Vec<String> {
        self.pending_node_losses
            .lock()
            .expect("worker loss lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub(crate) fn acknowledge_node_loss(&self, node_id: &str) {
        self.pending_node_losses
            .lock()
            .expect("worker loss lock poisoned")
            .remove(node_id);
    }

    pub(super) fn queue_node_loss(&self, node_id: &str) {
        self.acknowledge_probe_failure(node_id);
        self.pending_node_losses
            .lock()
            .expect("worker loss lock poisoned")
            .insert(node_id.to_owned());
    }

    pub(crate) fn pending_probe_failures(&self) -> Vec<String> {
        self.pending_probe_failures
            .lock()
            .expect("worker probe failure lock poisoned")
            .iter()
            .cloned()
            .collect()
    }

    pub(crate) fn acknowledge_probe_failure(&self, node_id: &str) {
        self.pending_probe_failures
            .lock()
            .expect("worker probe failure lock poisoned")
            .remove(node_id);
    }

    pub(super) fn queue_probe_failure(&self, node_id: &str) {
        self.pending_probe_failures
            .lock()
            .expect("worker probe failure lock poisoned")
            .insert(node_id.to_owned());
    }

    pub(super) fn mark_probe_failure(&self, target: &WorkerProbeTarget) {
        let mut entries = self.inner.lock().expect("worker registry lock poisoned");
        let Some(entry) = entries.get_mut(&target.connection.node_id) else {
            return;
        };
        if entry.connection_generation != target.generation {
            return;
        }
        let (queue_loss, queue_probe_failure) = if entry.snapshot.take().is_some() {
            entry.probe_failure = ProbeFailureConfirmation::Awaiting;
            (false, false)
        } else {
            match entry.probe_failure {
                ProbeFailureConfirmation::Clear => (false, true),
                ProbeFailureConfirmation::Awaiting => {
                    entry.probe_failure = ProbeFailureConfirmation::Confirmed;
                    (true, false)
                }
                ProbeFailureConfirmation::Confirmed => (false, false),
            }
        };
        drop(entries);
        if queue_loss {
            self.queue_node_loss(&target.connection.node_id);
        } else if queue_probe_failure {
            self.queue_probe_failure(&target.connection.node_id);
        }
    }
}
