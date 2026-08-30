use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerNode {
    pub node_id: String,
    pub cpu_capacity_millis: u64,
    pub cpu_used_millis: u64,
    pub memory_capacity_bytes: u64,
    pub memory_used_bytes: u64,
    pub execution_capacity: u32,
    pub execution_used: u32,
    pub queue_depth: u32,
    pub cached_content: BTreeSet<String>,
}

impl SchedulerNode {
    #[must_use]
    pub fn with_execution_slots(node_id: impl Into<String>, execution_capacity: u32) -> Self {
        Self {
            node_id: node_id.into(),
            cpu_capacity_millis: u64::MAX,
            cpu_used_millis: 0,
            memory_capacity_bytes: u64::MAX,
            memory_used_bytes: 0,
            execution_capacity,
            execution_used: 0,
            queue_depth: 0,
            cached_content: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_execution_usage(mut self, execution_used: u32) -> Self {
        self.execution_used = execution_used;
        self
    }

    #[must_use]
    pub fn with_cached_content(mut self, key: impl Into<String>) -> Self {
        self.cached_content.insert(key.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchCommand {
    pub run_id: String,
    pub job_id: String,
    pub node_id: String,
    pub authored_attempt: u32,
    pub dispatch_generation: u32,
    pub fencing_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptCompletion {
    Succeeded { terminal_digest: String },
    Failed { terminal_digest: String },
}

impl AttemptCompletion {
    pub(crate) fn digest(&self) -> &str {
        match self {
            Self::Succeeded { terminal_digest } | Self::Failed { terminal_digest } => {
                terminal_digest
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultAcceptance {
    Applied,
    Duplicate,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownOutcomeResolution {
    Retrying,
    Failed,
    Stale,
}
