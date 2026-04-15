use std::path::Path;

use super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in super::super) fn container_exit_code(&self, container_id: &str) -> i64 {
        self.container_exit_codes
            .lock()
            .expect("container exit codes lock")
            .get(container_id)
            .copied()
            .unwrap_or(1)
    }

    pub(in super::super) fn path_is_visible(&self, source: &Path) -> bool {
        self.visible_roots
            .iter()
            .any(|root| source.starts_with(root.as_path()))
    }
}
