mod containers;
mod images;
mod query;
mod removal;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, AtomicUsize},
};
use std::time::Duration;
use tokio::sync::Notify;

use super::{CreateRecord, DockerOperation};

pub(super) struct FakeDockerDaemonState {
    visible_roots: Vec<PathBuf>,
    daemon_arch: String,
    version_fails: bool,
    pub(super) wait_response_delay: Duration,
    pub(super) ping_response_delay: Duration,
    pub(super) memory_usage_bytes: u64,
    pub(super) oom_killed: bool,
    next_container_id: AtomicU64,
    pull_count: AtomicU64,
    present_images: Mutex<BTreeSet<String>>,
    create_records: Mutex<Vec<CreateRecord>>,
    container_exit_codes: Mutex<BTreeMap<String, i64>>,
    pause_after_list: Mutex<BTreeSet<String>>,
    removed_containers: Mutex<Vec<String>>,
    remove_notify: Notify,
    removal_failures: AtomicUsize,
    operations: Mutex<Vec<DockerOperation>>,
}

impl FakeDockerDaemonState {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        visible_roots: Vec<PathBuf>,
        image_present: bool,
        daemon_arch: String,
        version_fails: bool,
        wait_response_delay: Duration,
        ping_response_delay: Duration,
        memory_usage_bytes: u64,
        removal_failures: usize,
        oom_killed: bool,
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
            ping_response_delay,
            memory_usage_bytes,
            oom_killed,
            next_container_id: AtomicU64::new(1),
            pull_count: AtomicU64::new(0),
            present_images: Mutex::new(present_images),
            create_records: Mutex::new(Vec::new()),
            container_exit_codes: Mutex::new(BTreeMap::new()),
            pause_after_list: Mutex::new(BTreeSet::new()),
            removed_containers: Mutex::new(Vec::new()),
            remove_notify: Notify::new(),
            removal_failures: AtomicUsize::new(removal_failures),
            operations: Mutex::new(Vec::new()),
        }
    }
}
