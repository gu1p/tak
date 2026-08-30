use std::num::{NonZeroU32, NonZeroU64};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionScope {
    Run,
    Submitter,
    Project,
    Worktree,
    Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldMode {
    During,
    AtStart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueDiscipline {
    Fifo,
    Priority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueDefinition {
    pub name: String,
    pub scope: DefinitionScope,
    pub scope_key: Option<String>,
    pub max_parallel_tasks: NonZeroU32,
    pub discipline: QueueDiscipline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LimiterDefinition {
    Lock {
        name: String,
        scope: DefinitionScope,
        scope_key: Option<String>,
        hold: HoldMode,
    },
    RateLimit {
        name: String,
        scope: DefinitionScope,
        scope_key: Option<String>,
        burst: NonZeroU32,
        refill_millis_per_second: NonZeroU64,
    },
    ProcessCap {
        name: String,
        scope: DefinitionScope,
        scope_key: Option<String>,
        max_processes: NonZeroU32,
        hold: HoldMode,
    },
    Resource {
        name: String,
        scope: DefinitionScope,
        scope_key: Option<String>,
        capacity_millis: NonZeroU64,
        hold: HoldMode,
    },
}

impl LimiterDefinition {
    pub(super) fn name(&self) -> &str {
        match self {
            Self::Lock { name, .. }
            | Self::RateLimit { name, .. }
            | Self::ProcessCap { name, .. }
            | Self::Resource { name, .. } => name,
        }
    }

    pub(super) fn capacity_millis(&self) -> u64 {
        match self {
            Self::Lock { .. } => 1_000,
            Self::RateLimit { burst: permits, .. }
            | Self::ProcessCap {
                max_processes: permits,
                ..
            } => u64::from(permits.get()) * 1_000,
            Self::Resource {
                capacity_millis, ..
            } => capacity_millis.get(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimiterClaim {
    pub name: String,
    pub amount_millis: NonZeroU64,
}
