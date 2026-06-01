use anyhow::Result;
use serde::{Deserialize, Serialize};
use tak_core::model::TaskLabel;

use super::PlacementMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutputChunk {
    pub task_run_id: String,
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub stream: OutputStream,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusPhase {
    RemoteProbe,
    RemoteStageWorkspace,
    RemoteSubmit,
    RemoteWait,
    RemoteSyncOutputs,
    RetryWait,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatusEventKind {
    LocalDaemonConnection,
    RemoteCapacityDiscovery,
    RemoteNodeProbe,
    RemoteNodeConnected,
    RemoteNodeUnavailable,
    WorkspaceStage,
    UploadStart,
    UploadProgress,
    UploadComplete,
    WorkerSelected,
    QueueAdmission,
    QueuePositionChanged,
    ResourceMatch,
    Dispatch,
    RemoteExecutionStart,
    RetryScheduled,
    RecoverableFailure,
    FatalFailure,
    Cancellation,
    Completion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatusEvent {
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub phase: TaskStatusPhase,
    pub remote_node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStructuredStatusEvent {
    pub task_label: TaskLabel,
    pub operation_name: String,
    pub attempt: u32,
    pub phase: TaskStatusPhase,
    pub kind: TaskStatusEventKind,
    pub message: String,
    pub timestamp_ms: i64,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub local_daemon_path: Option<String>,
    pub transport: Option<String>,
    pub remote_node_id: Option<String>,
    pub queue_id: Option<String>,
    pub queue_position: Option<usize>,
    pub eligible_worker_count: Option<usize>,
    pub rejection_reason: Option<String>,
    pub original_error: Option<String>,
    pub retryable: Option<bool>,
    pub bytes_total: Option<u64>,
    pub bytes_sent: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStartedEvent {
    pub task_run_id: String,
    pub task_label: TaskLabel,
    pub placement_mode: PlacementMode,
    pub remote_node_id: Option<String>,
    pub origin: Option<String>,
    pub runtime: Option<String>,
    pub runtime_source: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFinishedEvent {
    pub task_run_id: String,
    pub task_label: TaskLabel,
    pub attempts: u32,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub placement_mode: PlacementMode,
    pub remote_node_id: Option<String>,
}

pub trait TaskOutputObserver: Send + Sync {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()>;

    fn observe_status(&self, _event: TaskStatusEvent) -> Result<()> {
        Ok(())
    }

    fn observe_structured_status(&self, _event: TaskStructuredStatusEvent) -> Result<()> {
        Ok(())
    }

    fn observe_task_started(&self, _event: TaskStartedEvent) -> Result<()> {
        Ok(())
    }

    fn observe_task_finished(&self, _event: TaskFinishedEvent) -> Result<()> {
        Ok(())
    }
}
