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
    RemoteWait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatusEvent {
    pub task_label: TaskLabel,
    pub attempt: u32,
    pub phase: TaskStatusPhase,
    pub remote_node_id: Option<String>,
    pub message: String,
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

    fn observe_task_finished(&self, _event: TaskFinishedEvent) -> Result<()> {
        Ok(())
    }
}
