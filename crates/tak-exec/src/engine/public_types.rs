#[derive(Debug, Clone)]
pub struct RunOptions {
    pub jobs: usize,
    pub keep_going: bool,
    pub lease_socket: Option<PathBuf>,
    pub lease_ttl_ms: u64,
    pub lease_poll_interval_ms: u64,
    pub session_id: Option<String>,
    pub user: Option<String>,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskRunResult {
    pub attempts: u32,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub placement_mode: PlacementMode,
    pub remote_node_id: Option<String>,
    pub remote_transport_kind: Option<String>,
    pub decision_reason: Option<String>,
    pub context_manifest_hash: Option<String>,
    pub remote_runtime_kind: Option<String>,
    pub remote_runtime_engine: Option<String>,
    pub remote_logs: Vec<RemoteLogChunk>,
    pub synced_outputs: Vec<SyncedOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteLogChunk {
    pub seq: u64,
    pub chunk: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedOutput {
    pub path: String,
    pub digest: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionSpec {
    pub steps: Vec<StepDef>,
    pub timeout_s: Option<u64>,
    pub runtime: Option<RemoteRuntimeSpec>,
    pub node_id: String,
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
