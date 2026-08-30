use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub seq: u64,
    pub kind: RunEventKind,
    pub job_id: Option<String>,
    pub task_ids: Vec<String>,
    pub node_id: Option<String>,
    pub message: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunDetails {
    pub summary: RunSummary,
    pub jobs: Vec<RunJobSummary>,
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
