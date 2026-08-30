use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::LimiterClaim;
use crate::v2::{Affinity, OutputSelector, RemoteSelection, Session, Step};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryPolicy {
    pub max_attempts: NonZeroU32,
    pub backoff_millis: u64,
    pub max_backoff_millis: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: NonZeroU32::MIN,
            backoff_millis: 0,
            max_backoff_millis: 0,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedJob {
    pub job_id: String,
    pub task_ids: Vec<String>,
    pub placement_candidates: Vec<PlacementCandidate>,
    pub retry: RetryPolicy,
    pub idempotent: bool,
    pub queue: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementPolicy {
    pub selection: RemoteSelection,
}
