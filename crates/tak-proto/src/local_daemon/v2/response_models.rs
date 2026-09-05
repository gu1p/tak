use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonStatusSnapshot {
    pub active_leases: usize,
    pub pending_requests: usize,
    pub limiter_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteInventoryEntry {
    pub node_id: String,
    pub display_name: String,
    pub base_url: String,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub transport: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemotePeerHealth {
    pub node_id: String,
    pub display_name: String,
    pub transport: String,
    pub endpoint: String,
    pub state: String,
    pub last_heartbeat_ms: Option<i64>,
    pub last_successful_connection_ms: Option<i64>,
    pub last_error_summary: Option<String>,
    pub active_job_count: Option<u32>,
    pub queue_depth: Option<u32>,
    pub resource_summary: Option<String>,
    pub protocol_version: Option<String>,
    pub heartbeat_rtt_ms: Option<u64>,
    pub reconnect_attempts: u32,
    pub pools: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteStatusEntry {
    pub remote: RemoteInventoryEntry,
    pub snapshot: Option<crate::worker_v2::WorkerSnapshot>,
    pub detail_base64: Option<String>,
    pub error: Option<String>,
    pub peer: Option<RemotePeerHealth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDisposition {
    Present,
    UploadRequired { next_offset: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunLifecycleState {
    AwaitingWorkspace,
    AwaitingCommit,
    Queued,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}

impl RunLifecycleState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingWorkspace => "awaiting_workspace",
            Self::AwaitingCommit => "awaiting_commit",
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Cancelling => "cancelling",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEventKind {
    Submitted,
    WorkspaceUploading,
    Queued,
    Transferring,
    Running,
    Retrying,
    OutputCommitting,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub seq: u64,
    pub kind: RunEventKind,
    pub job_id: Option<String>,
    pub task_ids: Vec<String>,
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_attempt: Option<u32>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSummary {
    pub run_id: String,
    pub state: RunLifecycleState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub targets: Vec<String>,
    pub total_jobs: u32,
    pub terminal_jobs: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunJobSummary {
    pub job_id: String,
    pub task_ids: Vec<String>,
    pub state: String,
    pub node_id: Option<String>,
    pub attempt: u32,
    pub cache: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placement_candidate_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunDetails {
    pub summary: RunSummary,
    pub jobs: Vec<RunJobSummary>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub max_parallel_jobs: u32,
    #[serde(default)]
    pub logs_expired: bool,
    #[serde(default)]
    pub outputs_expired: bool,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputArtifact {
    pub path: String,
    pub entry_type: String,
    pub executable: bool,
    pub symlink_target: Option<String>,
    pub size: u64,
    pub sha256: String,
    pub artifact_id: String,
}
