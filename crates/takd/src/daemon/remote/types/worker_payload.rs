use std::path::PathBuf;

use serde::Serialize;
use tak_core::model::{OutputSelectorSpec, RemoteRuntimeSpec, RetryDef, StepDef};
use tak_proto::SubmittedNeed;

#[derive(Debug, Clone)]
pub(in crate::daemon::remote) struct RemoteWorkerSubmitPayload {
    pub(in crate::daemon::remote) workspace_zip: Vec<u8>,
    pub(in crate::daemon::remote) task_run_id: String,
    pub(in crate::daemon::remote) task_label: String,
    pub(in crate::daemon::remote) attempt: u32,
    pub(in crate::daemon::remote) steps: Vec<StepDef>,
    pub(in crate::daemon::remote) timeout_s: Option<u64>,
    pub(in crate::daemon::remote) runtime: Option<RemoteRuntimeSpec>,
    pub(in crate::daemon::remote) needs: Vec<SubmittedNeed>,
    pub(in crate::daemon::remote) outputs: Vec<OutputSelectorSpec>,
    pub(in crate::daemon::remote) session: Option<RemoteWorkerSession>,
    pub(in crate::daemon::remote) fused_members: Vec<RemoteWorkerFusedMember>,
    pub(in crate::daemon::remote) origin: Option<String>,
    pub(in crate::daemon::remote) runtime_source: Option<String>,
    pub(in crate::daemon::remote) command: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::daemon::remote) struct RemoteWorkerFusedMember {
    pub(in crate::daemon::remote) task_label: String,
    pub(in crate::daemon::remote) steps: Vec<StepDef>,
    pub(in crate::daemon::remote) timeout_s: Option<u64>,
    pub(in crate::daemon::remote) retry: RetryDef,
}

#[derive(Debug, Clone)]
pub(in crate::daemon::remote) struct RemoteWorkerSession {
    pub(in crate::daemon::remote) key: String,
    pub(in crate::daemon::remote) reuse: RemoteWorkerSessionReuse,
}

#[derive(Debug, Clone)]
pub(in crate::daemon::remote) enum RemoteWorkerSessionReuse {
    ShareWorkspace,
    SharePaths { paths: Vec<OutputSelectorSpec> },
    Container,
}

#[derive(Debug, Clone)]
pub struct RemoteImageCacheRuntimeConfig {
    pub db_path: PathBuf,
    pub budget_bytes: u64,
    pub mutable_tag_ttl_secs: u64,
    pub sweep_interval_secs: u64,
    pub low_disk_min_free_percent: f64,
    pub low_disk_min_free_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::daemon::remote) struct RemoteWorkerOutputRecord {
    pub(in crate::daemon::remote) path: String,
    pub(in crate::daemon::remote) digest: String,
    pub(in crate::daemon::remote) size: u64,
}
