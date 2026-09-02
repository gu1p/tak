use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::LimiterClaim;
use crate::v2::{
    Affinity, OutputSelector, RemoteRequirements, RemoteSelection, Session, Step, TaskRuntime,
};

#[cfg(test)]
mod retry_tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementCandidate {
    pub node_id: String,
    pub kind: PlacementKind,
    pub transport: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub tier: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<RemoteRequirements>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryJitter {
    None,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicy {
    pub max_attempts: NonZeroU32,
    pub on_exit: Vec<i32>,
    pub backoff_millis: u64,
    pub max_backoff_millis: u64,
    pub jitter: RetryJitter,
}

impl RetryPolicy {
    #[must_use]
    pub fn allows_exit(&self, exit_code: Option<i32>) -> bool {
        self.on_exit.is_empty() || exit_code.is_some_and(|code| self.on_exit.contains(&code))
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: NonZeroU32::MIN,
            on_exit: Vec::new(),
            backoff_millis: 0,
            max_backoff_millis: 0,
            jitter: RetryJitter::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobContextManifest {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTaskUnit {
    pub task_id: String,
    pub job_id: String,
    pub dependencies: Vec<String>,
    pub steps: Vec<Step>,
    pub outputs: Vec<OutputSelector>,
    pub pass_env_names: Vec<String>,
    pub idempotent: bool,
    pub affinity: Option<Affinity>,
    pub timeout_s: Option<u64>,
    pub runtime: Option<TaskRuntime>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedJob {
    pub job_id: String,
    pub task_ids: Vec<String>,
    pub placement_policy: PlacementPolicy,
    pub placement_candidates: Vec<PlacementCandidate>,
    pub resources: ResourceRequest,
    pub retry: RetryPolicy,
    pub idempotent: bool,
    pub queue: Option<String>,
    pub queue_slots: NonZeroU32,
    pub queue_priority: i32,
    pub limiter_claims: Vec<LimiterClaim>,
    pub affinity: Option<Affinity>,
    pub session: Option<Session>,
    pub context_manifest: JobContextManifest,
    pub pass_env_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobEdge {
    pub dependency_job_id: String,
    pub dependent_job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementPolicy {
    pub policy_id: String,
    pub selection: RemoteSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    pub cpu_millis: u64,
    pub memory_bytes: u64,
    pub execution_slots: NonZeroU32,
}

impl Default for ResourceRequest {
    fn default() -> Self {
        Self {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: NonZeroU32::MIN,
        }
    }
}
