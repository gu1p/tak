use std::num::{NonZeroU32, NonZeroU64};

use serde::{Deserialize, Serialize};

use crate::v2::{DefinitionScope, HoldMode, QueueDiscipline};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredQueueUse {
    pub name: String,
    pub scope: DefinitionScope,
    pub slots: NonZeroU32,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredLimiterClaim {
    pub name: String,
    pub scope: DefinitionScope,
    pub amount_millis: NonZeroU64,
    pub hold: HoldMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredLimiterDefinition {
    Lock {
        name: String,
        scope: DefinitionScope,
    },
    RateLimit {
        name: String,
        scope: DefinitionScope,
        burst: NonZeroU32,
        refill_millis_per_second: NonZeroU64,
    },
    ProcessCap {
        name: String,
        scope: DefinitionScope,
        max_processes: NonZeroU32,
    },
    Resource {
        name: String,
        scope: DefinitionScope,
        capacity_millis: NonZeroU64,
    },
}

impl AuthoredLimiterDefinition {
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Lock { name, .. }
            | Self::RateLimit { name, .. }
            | Self::ProcessCap { name, .. }
            | Self::Resource { name, .. } => name,
        }
    }

    #[must_use]
    pub fn scope(&self) -> &DefinitionScope {
        match self {
            Self::Lock { scope, .. }
            | Self::RateLimit { scope, .. }
            | Self::ProcessCap { scope, .. }
            | Self::Resource { scope, .. } => scope,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredQueueDefinition {
    pub name: String,
    pub scope: DefinitionScope,
    pub max_parallel_tasks: NonZeroU32,
    pub discipline: QueueDiscipline,
}
