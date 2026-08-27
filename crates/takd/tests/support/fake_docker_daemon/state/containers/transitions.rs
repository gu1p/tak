use super::super::super::DockerOperation;
use super::super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in super::super::super) fn pause_container_after_next_list(&self, container_id: &str) {
        self.pause_after_list
            .lock()
            .expect("pause after list lock")
            .insert(container_id.to_string());
    }

    pub(in super::super::super) fn apply_post_list_transitions(&self) {
        let paused =
            std::mem::take(&mut *self.pause_after_list.lock().expect("pause after list lock"));
        for record in self
            .create_records
            .lock()
            .expect("create records lock")
            .iter_mut()
        {
            if paused.contains(&record.container_id) {
                record.state = "paused".to_string();
            }
        }
    }

    pub(in super::super::super) fn container_state(&self, container_id: &str) -> Option<String> {
        if !self.container_exists(container_id) {
            return None;
        }
        self.create_records
            .lock()
            .expect("create records lock")
            .iter()
            .find(|record| record.container_id == container_id)
            .map(|record| record.state.clone())
    }

    pub(in super::super::super) fn container_exists(&self, container_id: &str) -> bool {
        let removed = self
            .removed_containers
            .lock()
            .expect("removed containers lock");
        !removed.iter().any(|removed| removed == container_id)
            && self
                .create_records
                .lock()
                .expect("create records lock")
                .iter()
                .any(|record| record.container_id == container_id)
    }

    pub(in super::super::super) fn record_container_unpause_attempt(&self, container_id: &str) {
        self.operations
            .lock()
            .expect("operations lock")
            .push(DockerOperation::UnpauseAttempted(container_id.to_string()));
    }
}
