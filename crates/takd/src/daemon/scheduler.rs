use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use tak_proto::worker_v2::WorkerProcessObservation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerNode {
    pub node_id: String,
    pub transport: Option<String>,
    pub pools: BTreeSet<String>,
    pub tags: BTreeSet<String>,
    pub capabilities: BTreeSet<String>,
    pub cpu_capacity_millis: u64,
    pub cpu_used_millis: u64,
    pub memory_capacity_bytes: u64,
    pub memory_used_bytes: u64,
    pub execution_capacity: u32,
    pub execution_used: u32,
    pub queue_depth: u32,
    pub cached_content: BTreeSet<String>,
    pub processes: Vec<WorkerProcessObservation>,
}

impl SchedulerNode {
    #[must_use]
    pub fn with_execution_slots(node_id: impl Into<String>, execution_capacity: u32) -> Self {
        let node_id = node_id.into();
        Self {
            transport: (node_id != "local").then(|| "direct".to_owned()),
            node_id,
            pools: BTreeSet::new(),
            tags: BTreeSet::new(),
            capabilities: BTreeSet::new(),
            cpu_capacity_millis: u64::MAX,
            cpu_used_millis: 0,
            memory_capacity_bytes: u64::MAX,
            memory_used_bytes: 0,
            execution_capacity,
            execution_used: 0,
            queue_depth: 0,
            cached_content: BTreeSet::new(),
            processes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttemptCompletion {
    Succeeded {
        terminal_digest: String,
    },
    SucceededWithRuntime {
        terminal_digest: String,
        runtime: AttemptRuntimeMetadata,
    },
    Failed {
        terminal_digest: String,
        exit_code: Option<i32>,
    },
    FailedWithRuntime {
        terminal_digest: String,
        exit_code: Option<i32>,
        runtime: AttemptRuntimeMetadata,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptRuntimeMetadata {
    pub kind: String,
    pub engine: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutputStream {
    Stdout,
    Stderr,
}

impl AttemptCompletion {
    pub(crate) fn digest(&self) -> &str {
        match self {
            Self::Succeeded { terminal_digest }
            | Self::SucceededWithRuntime {
                terminal_digest, ..
            }
            | Self::Failed {
                terminal_digest, ..
            }
            | Self::FailedWithRuntime {
                terminal_digest, ..
            } => terminal_digest,
        }
    }

    pub(crate) fn exit_code(&self) -> Option<i32> {
        match self {
            Self::Succeeded { .. } | Self::SucceededWithRuntime { .. } => Some(0),
            Self::Failed { exit_code, .. } | Self::FailedWithRuntime { exit_code, .. } => {
                *exit_code
            }
        }
    }

    pub(crate) fn succeeded(&self) -> bool {
        matches!(
            self,
            Self::Succeeded { .. } | Self::SucceededWithRuntime { .. }
        )
    }

    pub(crate) fn runtime(&self) -> Option<&AttemptRuntimeMetadata> {
        match self {
            Self::SucceededWithRuntime { runtime, .. }
            | Self::FailedWithRuntime { runtime, .. } => Some(runtime),
            Self::Succeeded { .. } | Self::Failed { .. } => None,
        }
    }

    pub(crate) fn with_runtime(self, runtime: Option<AttemptRuntimeMetadata>) -> Self {
        let Some(runtime) = runtime else {
            return self;
        };
        match self {
            Self::Succeeded { terminal_digest } => Self::SucceededWithRuntime {
                terminal_digest,
                runtime,
            },
            Self::Failed {
                terminal_digest,
                exit_code,
            } => Self::FailedWithRuntime {
                terminal_digest,
                exit_code,
                runtime,
            },
            value @ (Self::SucceededWithRuntime { .. } | Self::FailedWithRuntime { .. }) => value,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLossResolution {
    Applied,
    Duplicate,
}
