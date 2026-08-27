use std::sync::atomic::Ordering;

use super::super::DockerOperation;
use super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in super::super) fn operations(&self) -> Vec<DockerOperation> {
        self.operations.lock().expect("operations lock").clone()
    }

    pub(in super::super) fn record_container_removal_attempt(&self, container_id: &str) -> bool {
        self.operations
            .lock()
            .expect("operations lock")
            .push(DockerOperation::RemovalAttempted(container_id.to_string()));
        let failure_injected = self
            .removal_failures
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok();
        !failure_injected
    }

    pub(in super::super) fn removed_containers(&self) -> Vec<String> {
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .clone()
    }

    pub(in super::super) fn record_container_removed(&self, container_id: &str) {
        self.operations
            .lock()
            .expect("operations lock")
            .push(DockerOperation::Removed(container_id.to_string()));
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .push(container_id.to_string());
        self.remove_notify.notify_waiters();
    }

    pub(in super::super) async fn wait_for_exit_or_remove(&self, container_id: &str) -> bool {
        if self.wait_response_delay.is_zero() {
            return false;
        }
        let sleep = tokio::time::sleep(self.wait_response_delay);
        tokio::pin!(sleep);
        loop {
            if self
                .removed_containers
                .lock()
                .expect("removed containers lock")
                .iter()
                .any(|removed| removed == container_id)
            {
                return true;
            }
            tokio::select! {
                _ = &mut sleep => return false,
                _ = self.remove_notify.notified() => {}
            }
        }
    }
}
