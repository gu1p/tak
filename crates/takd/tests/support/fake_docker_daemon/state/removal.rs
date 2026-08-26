use super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in super::super) fn removed_containers(&self) -> Vec<String> {
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .clone()
    }

    pub(in super::super) fn record_container_removed(&self, container_id: &str) {
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .push(container_id.to_string());
        self.remove_notify.notify_waiters();
    }

    pub(in super::super) async fn wait_for_exit_or_remove(&self, container_id: &str) {
        if self.wait_response_delay.is_zero() {
            return;
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
                return;
            }
            tokio::select! {
                _ = &mut sleep => return,
                _ = self.remove_notify.notified() => {}
            }
        }
    }
}
