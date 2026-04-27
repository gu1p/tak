use std::sync::atomic::Ordering;

use super::{BuildRecord, CreateRecord, FakeDockerDaemonState};

impl FakeDockerDaemonState {
    pub(super) async fn wait_until_released(&self) {
        loop {
            if self.release_requested.load(Ordering::SeqCst) {
                return;
            }
            self.release_notify.notified().await;
        }
    }

    pub(super) fn record_build(&self, build: BuildRecord) {
        self.builds.lock().expect("build records lock").push(build);
    }

    pub(super) fn record_create(&self, create: CreateRecord) {
        self.creates
            .lock()
            .expect("create records lock")
            .push(create);
    }
}
