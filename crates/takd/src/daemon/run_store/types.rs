use std::path::PathBuf;

use tak_proto::local_daemon::v2::WorkspaceDisposition;

#[derive(Clone)]
pub struct RunStore {
    pub(super) db_path: PathBuf,
    pub(super) blob_root: PathBuf,
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
