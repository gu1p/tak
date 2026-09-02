use serde::{Deserialize, Serialize};

use super::{Session, TaskRuntime};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteSelection {
    #[default]
    Balanced,
    Sequential,
    RoundRobin,
}

impl RemoteSelection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
            Self::Sequential => "sequential",
            Self::RoundRobin => "round_robin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalExecution {
    #[serde(default)]
    pub reason: String,
    pub session: Option<Box<Session>>,
    pub runtime: Option<TaskRuntime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteExecution {
    #[serde(default)]
    pub reason: String,
    pub pool: Option<String>,
    pub required_tags: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub transport: Option<String>,
    pub selection: RemoteSelection,
    pub session: Option<Box<Session>>,
    pub runtime: Option<TaskRuntime>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteRequirements {
    pub pool: Option<String>,
    pub required_tags: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub transport: Option<String>,
}

impl From<&RemoteExecution> for RemoteRequirements {
    fn from(remote: &RemoteExecution) -> Self {
        Self {
            pool: remote.pool.clone(),
            required_tags: remote.required_tags.clone(),
            required_capabilities: remote.required_capabilities.clone(),
            transport: remote
                .transport
                .as_ref()
                .filter(|transport| transport.as_str() != "any")
                .cloned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Execution {
    LocalOnly {
        local: LocalExecution,
    },
    RemoteOnly {
        remote: RemoteExecution,
    },
    FirstAvailable {
        policy_id: String,
        placements: Vec<Execution>,
    },
}

impl Execution {
    #[must_use]
    pub fn remote(&self) -> Option<&RemoteExecution> {
        match self {
            Self::RemoteOnly { remote } => Some(remote),
            Self::LocalOnly { .. } | Self::FirstAvailable { .. } => None,
        }
    }

    #[must_use]
    pub fn runtime(&self) -> Option<&TaskRuntime> {
        match self {
            Self::RemoteOnly { remote } => remote.runtime.as_ref(),
            Self::LocalOnly { local } => local.runtime.as_ref(),
            Self::FirstAvailable { placements, .. } => placements.first().and_then(Self::runtime),
        }
    }
}
