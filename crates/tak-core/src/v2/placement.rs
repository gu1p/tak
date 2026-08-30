use serde::{Deserialize, Serialize};

use super::Session;

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
    pub session: Option<Box<Session>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteExecution {
    pub pool: Option<String>,
    pub required_tags: Vec<String>,
    pub required_capabilities: Vec<String>,
    pub transport: Option<String>,
    pub selection: RemoteSelection,
    pub session: Option<Box<Session>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Execution {
    LocalOnly { local: LocalExecution },
    RemoteOnly { remote: RemoteExecution },
}

impl Execution {
    #[must_use]
    pub fn remote(&self) -> Option<&RemoteExecution> {
        match self {
            Self::RemoteOnly { remote } => Some(remote),
            Self::LocalOnly { .. } => None,
        }
    }
}
