#[path = "state/image.rs"]
mod image;
#[path = "state/query.rs"]
mod query;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tokio::sync::Notify;

use super::{BuildRecord, CreateRecord, PullRecord};
pub(in crate::support::fake_docker_daemon) use image::ImageDeleteResult;

pub(super) struct FakeDockerDaemonState {
    pub(super) release_requested: AtomicBool,
    pub(super) release_notify: Notify,
    pub(super) image_refs: Mutex<BTreeMap<String, String>>,
    pub(super) images: Mutex<BTreeMap<String, u64>>,
    pub(super) builds: Mutex<Vec<BuildRecord>>,
    pub(super) creates: Mutex<Vec<CreateRecord>>,
    pub(super) pulls: Mutex<Vec<PullRecord>>,
    pub(super) image_removal_attempts: Mutex<Vec<String>>,
    image_removal_failure_status: Mutex<Option<u16>>,
}

impl FakeDockerDaemonState {
    pub(super) fn new() -> Self {
        let mut image_refs = BTreeMap::new();
        image_refs.insert("alpine:3.20".to_string(), super::IMAGE_ID.to_string());
        let mut images = BTreeMap::new();
        images.insert(super::IMAGE_ID.to_string(), 1024);
        Self {
            release_requested: AtomicBool::new(false),
            release_notify: Notify::new(),
            image_refs: Mutex::new(image_refs),
            images: Mutex::new(images),
            builds: Mutex::new(Vec::new()),
            creates: Mutex::new(Vec::new()),
            pulls: Mutex::new(Vec::new()),
            image_removal_attempts: Mutex::new(Vec::new()),
            image_removal_failure_status: Mutex::new(None),
        }
    }

    pub(super) async fn wait_until_released(&self) {
        loop {
            if self.release_requested.load(Ordering::SeqCst) {
                return;
            }
            self.release_notify.notified().await;
        }
    }

    pub(super) fn record_build(&self, build: BuildRecord) {
        if !build.image_tag.is_empty() {
            self.set_image(&build.image_tag, super::IMAGE_ID, 1024);
        }
        self.builds.lock().expect("build records lock").push(build);
    }

    pub(super) fn record_create(&self, create: CreateRecord) {
        self.creates
            .lock()
            .expect("create records lock")
            .push(create);
    }

    pub(super) fn record_pull(&self, pull: PullRecord) {
        if !pull.image.is_empty() && self.image_info(&pull.image).is_none() {
            self.set_image(&pull.image, super::IMAGE_ID, 1024);
        }
        self.pulls.lock().expect("pull records lock").push(pull);
    }
}
