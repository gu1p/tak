use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use tak_proto::SubmitTaskRequest;

#[derive(Clone, Default)]
pub struct RecordingEvents {
    entries: Arc<Mutex<Vec<String>>>,
    submit_payloads: Arc<Mutex<Vec<SubmitTaskRequest>>>,
    uploads: Arc<Mutex<UploadState>>,
}

/// Workspace-upload bookkeeping, kept separate from `entries` so the ordered event log
/// (`snapshot`, used by lease/order tests) is unaffected by upload traffic.
#[derive(Default)]
struct UploadState {
    /// `upload_id`s seen at each `begin`, in order — the count is how many uploads happened.
    begins: Vec<String>,
    /// Completed upload_ids currently present on the node. A submit referencing an id not here
    /// is answered with 409 (mirrors a reaped blob); `reap_upload` removes one.
    available: HashSet<String>,
    /// `upload_id`s of submits rejected with 409 because the referenced blob was missing.
    conflicts: Vec<String>,
}

impl RecordingEvents {
    pub fn record(&self, entry: impl Into<String>) {
        self.entries.lock().expect("event lock").push(entry.into());
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.entries.lock().expect("event lock").clone()
    }

    pub fn record_submit_payload(&self, payload: SubmitTaskRequest) {
        self.submit_payloads
            .lock()
            .expect("payload lock")
            .push(payload);
    }

    pub fn submit_payloads(&self) -> Vec<SubmitTaskRequest> {
        self.submit_payloads.lock().expect("payload lock").clone()
    }

    fn uploads(&self) -> MutexGuard<'_, UploadState> {
        self.uploads.lock().expect("upload state lock")
    }

    pub(super) fn record_upload_begin(&self, upload_id: &str) {
        self.uploads().begins.push(upload_id.to_string());
    }

    pub(super) fn mark_upload_available(&self, upload_id: &str) {
        self.uploads().available.insert(upload_id.to_string());
    }

    pub(super) fn is_upload_available(&self, upload_id: &str) -> bool {
        self.uploads().available.contains(upload_id)
    }

    pub(super) fn record_upload_conflict(&self, upload_id: &str) {
        self.uploads().conflicts.push(upload_id.to_string());
    }

    /// Number of `begin` requests — i.e. how many times a workspace was actually uploaded.
    /// With per-job caching, identical content uploads exactly once.
    pub fn upload_begin_count(&self) -> usize {
        self.uploads().begins.len()
    }

    /// The `upload_id`s the node issued at `begin`, in order.
    pub fn upload_begin_ids(&self) -> Vec<String> {
        self.uploads().begins.clone()
    }

    /// The `upload_id`s of submits the node rejected with 409 (referenced a reaped blob).
    pub fn upload_conflicts(&self) -> Vec<String> {
        self.uploads().conflicts.clone()
    }

    /// Simulates the cleanup janitor reaping a stored blob: a later submit referencing it gets
    /// a 409 so the client must re-upload.
    pub fn reap_upload(&self, upload_id: &str) {
        self.uploads().available.remove(upload_id);
    }
}
