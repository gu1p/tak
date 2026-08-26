use std::sync::atomic::Ordering;

use super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in super::super) fn pull_count(&self) -> u64 {
        self.pull_count.load(Ordering::SeqCst)
    }

    pub(in super::super) fn daemon_arch(&self) -> &str {
        &self.daemon_arch
    }

    pub(in super::super) fn version_fails(&self) -> bool {
        self.version_fails
    }

    pub(in super::super) fn next_container_id(&self) -> String {
        let id = self.next_container_id.fetch_add(1, Ordering::SeqCst);
        format!("container-{id}")
    }

    pub(in super::super) fn image_present(&self, image: &str) -> bool {
        self.present_images
            .lock()
            .expect("present images lock")
            .contains(image)
    }

    pub(in super::super) fn mark_image_pulled(&self, image: &str) {
        self.present_images
            .lock()
            .expect("present images lock")
            .insert(image.to_string());
        self.pull_count.fetch_add(1, Ordering::SeqCst);
    }

    pub(in super::super) fn mark_image_built(&self, image: &str) {
        self.present_images
            .lock()
            .expect("present images lock")
            .insert(image.to_string());
    }
}
