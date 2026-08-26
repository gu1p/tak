mod containers;
mod images;
mod query;
mod removal;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, atomic::AtomicU64};
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
}
