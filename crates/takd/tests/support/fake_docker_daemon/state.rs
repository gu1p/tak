use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::CreateRecord;

pub(super) struct FakeDockerDaemonState {
    visible_roots: Vec<PathBuf>,
    image_present: AtomicBool,
    next_container_id: AtomicU64,
    pull_count: AtomicU64,
    create_records: Mutex<Vec<CreateRecord>>,
    container_exit_codes: Mutex<BTreeMap<String, i64>>,
}

impl FakeDockerDaemonState {
    pub(super) fn new(visible_roots: Vec<PathBuf>, image_present: bool) -> Self {
        Self {
            visible_roots,
            image_present: AtomicBool::new(image_present),
            next_container_id: AtomicU64::new(1),
            pull_count: AtomicU64::new(0),
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

    pub(super) fn next_container_id(&self) -> String {
        let id = self.next_container_id.fetch_add(1, Ordering::SeqCst);
        format!("container-{id}")
    }

    pub(super) fn image_present(&self) -> bool {
        self.image_present.load(Ordering::SeqCst)
    }

    pub(super) fn mark_image_pulled(&self) {
        self.image_present.store(true, Ordering::SeqCst);
        self.pull_count.fetch_add(1, Ordering::SeqCst);
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

    pub(super) fn container_exit_code(&self, container_id: &str) -> i64 {
        self.container_exit_codes
            .lock()
            .expect("container exit codes lock")
            .get(container_id)
            .copied()
            .unwrap_or(1)
    }

    pub(super) fn path_is_visible(&self, source: &Path) -> bool {
        self.visible_roots
            .iter()
            .any(|root| source.starts_with(root.as_path()))
    }
}
