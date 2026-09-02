use std::collections::BTreeMap;
use std::path::PathBuf;

use tak_core::model::{RemoteRuntimeSpec, StepDef, TaskLabel};

mod observer;
mod remote_worker_outcome;
mod remote_worker_spec_debug;

pub use observer::{
    OutputStream, TaskFinishedEvent, TaskOutputChunk, TaskOutputObserver, TaskStatusEvent,
    TaskStatusPhase,
};

#[derive(Clone)]
pub struct RemoteWorkerExecutionSpec {
    pub task_label: TaskLabel,
    pub task_run_id: String,
    pub attempt: u32,
    pub steps: Vec<StepDef>,
    pub base_environment: BTreeMap<String, String>,
    pub clear_environment: bool,
    pub timeout_s: Option<u64>,
    pub runtime: Option<RemoteRuntimeSpec>,
    pub node_id: String,
    pub container_user: Option<String>,
    pub image_cache: Option<ImageCacheOptions>,
    pub container_identity: Option<ContainerExecutionIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerExecutionIdentity {
    pub owner: String,
    pub submit_key: String,
    pub task_run_id: String,
}

#[derive(Debug, Clone)]
pub struct RemoteWorkerExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub runtime_kind: Option<String>,
    pub runtime_engine: Option<String>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RemoteWorkerExecutionOutcome {
    result: RemoteWorkerExecutionResult,
    container_oom_killed: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ImageCacheOptions {
    pub db_path: PathBuf,
    pub budget_bytes: u64,
    pub mutable_tag_ttl_secs: u64,
    pub sweep_interval_secs: u64,
    pub low_disk_min_free_percent: f64,
    pub low_disk_min_free_bytes: u64,
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
