use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use tak_core::model::{RemoteRuntimeSpec, StepDef, TaskLabel};

#[derive(Clone)]
pub struct RunOptions {
    pub jobs: usize,
    pub keep_going: bool,
    pub lease_socket: Option<PathBuf>,
    pub lease_ttl_ms: u64,
    pub lease_poll_interval_ms: u64,
    pub session_id: Option<String>,
    pub user: Option<String>,
    pub output_observer: Option<std::sync::Arc<dyn TaskOutputObserver>>,
}

impl std::fmt::Debug for RunOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunOptions")
            .field("jobs", &self.jobs)
            .field("keep_going", &self.keep_going)
            .field("lease_socket", &self.lease_socket)
            .field("lease_ttl_ms", &self.lease_ttl_ms)
            .field("lease_poll_interval_ms", &self.lease_poll_interval_ms)
            .field("session_id", &self.session_id)
            .field("user", &self.user)
            .field(
                "output_observer",
                &self.output_observer.as_ref().map(|_| "configured"),
            )
            .finish()
    }
}

impl Default for RunOptions {
    /// Returns conservative defaults for local execution and optional lease coordination.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            jobs: 1,
            keep_going: false,
            lease_socket: None,
            lease_ttl_ms: 30_000,
            lease_poll_interval_ms: 200,
            session_id: None,
            user: None,
            output_observer: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutputChunk {
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub stream: OutputStream,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatusPhase {
    RemoteProbe,
    RemoteStageWorkspace,
    RemoteSubmit,
    RemoteWait,
    RemoteSyncOutputs,
    RetryWait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatusEvent {
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub phase: TaskStatusPhase,
    pub remote_node_id: Option<String>,
    pub message: String,
}

pub trait TaskOutputObserver: Send + Sync {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()>;

    fn observe_status(&self, _event: TaskStatusEvent) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TaskRunResult {
    pub attempts: u32,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub failure_detail: Option<String>,
    pub placement_mode: PlacementMode,
    pub remote_node_id: Option<String>,
    pub remote_transport_kind: Option<String>,
    pub decision_reason: Option<String>,
    pub context_manifest_hash: Option<String>,
    pub remote_runtime_kind: Option<String>,
    pub remote_runtime_engine: Option<String>,
    pub session_name: Option<String>,
    pub session_reuse: Option<String>,
    pub remote_logs: Vec<RemoteLogChunk>,
    pub synced_outputs: Vec<SyncedOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLogChunk {
    pub seq: u64,
    pub stream: OutputStream,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedOutput {
    pub path: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionSpec {
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub steps: Vec<StepDef>,
    pub timeout_s: Option<u64>,
    pub runtime: Option<RemoteRuntimeSpec>,
    pub node_id: String,
    pub container_user: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub runtime_kind: Option<String>,
    pub runtime_engine: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementMode {
    Local,
    Remote,
}

impl PlacementMode {
    /// Returns a stable lowercase placement mode marker for CLI/user output.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub results: BTreeMap<TaskLabel, TaskRunResult>,
}
