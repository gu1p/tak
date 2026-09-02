use std::path::PathBuf;

use tak_proto::local_daemon::v2::{OutputArtifact, RunEvent, RunSummary, WorkspaceDisposition};

#[derive(Clone)]
pub struct RunStore {
    pub(super) db_path: PathBuf,
    pub(super) blob_root: PathBuf,
    pub(super) maintenance: RunStoreMaintenanceConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunStoreMaintenanceConfig {
    pub terminal_payload_retention: std::time::Duration,
    pub terminal_metadata_retention: std::time::Duration,
    pub workspace_path_blob_budget_bytes: u64,
    pub sweep_interval: std::time::Duration,
}

impl Default for RunStoreMaintenanceConfig {
    fn default() -> Self {
        Self {
            terminal_payload_retention: std::time::Duration::from_secs(7 * 24 * 60 * 60),
            terminal_metadata_retention: std::time::Duration::from_secs(30 * 24 * 60 * 60),
            workspace_path_blob_budget_bytes: 20 * 1024 * 1024 * 1024,
            sweep_interval: std::time::Duration::from_secs(60 * 60),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStoreMaintenanceReport {
    pub expired_payloads: u64,
    pub purged_runs: u64,
    pub evicted_workspace_path_blobs: u64,
    pub reclaimed_workspace_path_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitRunResult {
    pub run_id: String,
    pub workspace: WorkspaceDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadProgress {
    pub chunk_accepted: bool,
    pub next_offset: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputArtifactChunk {
    pub bytes: Vec<u8>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutputManifest {
    pub expired: bool,
    pub artifacts: Vec<OutputArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAttachmentSnapshot {
    pub summary: RunSummary,
    pub events: Vec<RunEvent>,
    pub next_event: u64,
    pub has_more: bool,
    pub logs_expired: bool,
}
