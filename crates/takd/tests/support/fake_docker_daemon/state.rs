mod query;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use super::CreateRecord;

pub(super) struct FakeDockerDaemonState {
    visible_roots: Vec<PathBuf>,
    daemon_arch: String,
    version_fails: bool,
    next_container_id: AtomicU64,
    pull_count: AtomicU64,
    present_images: Mutex<BTreeSet<String>>,
    create_records: Mutex<Vec<CreateRecord>>,
    container_exit_codes: Mutex<BTreeMap<String, i64>>,
}

impl FakeDockerDaemonState {
    pub(super) fn new(
        visible_roots: Vec<PathBuf>,
        image_present: bool,
        daemon_arch: String,
        version_fails: bool,
    ) -> Self {
        let mut present_images = BTreeSet::new();
        if image_present {
            present_images.insert("alpine:3.20".to_string());
        }
        Self {
            visible_roots,
            daemon_arch,
            version_fails,
            next_container_id: AtomicU64::new(1),
            pull_count: AtomicU64::new(0),
            present_images: Mutex::new(present_images),
            create_records: Mutex::new(Vec::new()),
            container_exit_codes: Mutex::new(BTreeMap::new()),
        }
    }

    pub(super) fn create_records(&self) -> Vec<CreateRecord> {
        self.create_records
            .lock()
            .expect("create records lock")
            .clone()
    }

    pub(super) fn pull_count(&self) -> u64 {
        self.pull_count.load(Ordering::SeqCst)
    }
    pub(super) fn daemon_arch(&self) -> &str {
        &self.daemon_arch
    }
    pub(super) fn version_fails(&self) -> bool {
        self.version_fails
    }
    pub(super) fn next_container_id(&self) -> String {
        let id = self.next_container_id.fetch_add(1, Ordering::SeqCst);
        format!("container-{id}")
    }
    pub(super) fn image_present(&self, image: &str) -> bool {
        self.present_images
            .lock()
            .expect("present images lock")
            .contains(image)
    }
    pub(super) fn mark_image_pulled(&self, image: &str) {
        self.present_images
            .lock()
            .expect("present images lock")
            .insert(image.to_string());
        self.pull_count.fetch_add(1, Ordering::SeqCst);
    }
    pub(super) fn mark_image_built(&self, image: &str) {
        self.present_images
            .lock()
            .expect("present images lock")
            .insert(image.to_string());
    }
    pub(super) fn record_create(&self, record: CreateRecord, exit_code: i64) {
        self.container_exit_codes
            .lock()
            .expect("container exit codes lock")
            .insert(record.container_id.clone(), exit_code);
        self.create_records
            .lock()
            .expect("create records lock")
            .push(record);
    }
}
