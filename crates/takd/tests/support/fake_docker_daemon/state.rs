mod query;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::sync::Notify;

use super::CreateRecord;

pub(super) struct FakeDockerDaemonState {
    visible_roots: Vec<PathBuf>,
    daemon_arch: String,
    version_fails: bool,
    pub(super) wait_response_delay: Duration,
    next_container_id: AtomicU64,
    pull_count: AtomicU64,
    present_images: Mutex<BTreeSet<String>>,
    create_records: Mutex<Vec<CreateRecord>>,
    container_exit_codes: Mutex<BTreeMap<String, i64>>,
    removed_containers: Mutex<Vec<String>>,
    remove_notify: Notify,
}

impl FakeDockerDaemonState {
    pub(super) fn new(
        visible_roots: Vec<PathBuf>,
        image_present: bool,
        daemon_arch: String,
        version_fails: bool,
        wait_response_delay: Duration,
    ) -> Self {
        let mut present_images = BTreeSet::new();
        if image_present {
            present_images.insert("alpine:3.20".to_string());
        }
        Self {
            visible_roots,
            daemon_arch,
            version_fails,
            wait_response_delay,
            next_container_id: AtomicU64::new(1),
            pull_count: AtomicU64::new(0),
            present_images: Mutex::new(present_images),
            create_records: Mutex::new(Vec::new()),
            container_exit_codes: Mutex::new(BTreeMap::new()),
            removed_containers: Mutex::new(Vec::new()),
            remove_notify: Notify::new(),
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
    pub(super) fn removed_containers(&self) -> Vec<String> {
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .clone()
    }
    pub(super) fn record_container_removed(&self, container_id: &str) {
        self.removed_containers
            .lock()
            .expect("removed containers lock")
            .push(container_id.to_string());
        self.remove_notify.notify_waiters();
    }

    pub(super) fn add_container(&self, container_id: &str, labels: BTreeMap<String, String>) {
        self.add_container_with_state(container_id, labels, "running");
    }

    pub(super) fn add_container_with_state(
        &self,
        container_id: &str,
        labels: BTreeMap<String, String>,
        state: &str,
    ) {
        self.record_create(
            CreateRecord {
                container_id: container_id.to_string(),
                image: Some("alpine:3.20".to_string()),
                cmd: vec!["sleep".to_string(), "60".to_string()],
                user: None,
                working_dir: None,
                binds: Vec::new(),
                labels,
                state: state.to_string(),
            },
            0,
        );
    }

    pub(super) fn container_summaries(&self) -> Vec<CreateRecord> {
        let removed = self
            .removed_containers
            .lock()
            .expect("removed containers lock")
            .clone();
        self.create_records
            .lock()
            .expect("create records lock")
            .iter()
            .filter(|record| !removed.contains(&record.container_id))
            .cloned()
            .collect()
    }
    pub(super) async fn wait_for_exit_or_remove(&self, container_id: &str) {
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
