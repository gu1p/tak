use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{
    JobEdge, LimiterDefinition, QueueDefinition, ResolvedJob, ResolvedRunError, ResolvedTaskUnit,
    WorkspaceDescriptor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedRunOptions {
    pub max_parallel_jobs: NonZeroU32,
    pub keep_going: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedRun {
    pub project_id: String,
    pub targets: Vec<String>,
    pub options: ResolvedRunOptions,
    pub workspace: WorkspaceDescriptor,
    pub tasks: Vec<ResolvedTaskUnit>,
    pub jobs: Vec<ResolvedJob>,
    pub job_edges: Vec<JobEdge>,
    pub limiter_definitions: Vec<LimiterDefinition>,
    pub queue_definitions: Vec<QueueDefinition>,
}

impl ResolvedRun {
    pub fn validate(&self) -> Result<(), ResolvedRunError> {
        super::validation::validate(self)
    }
}
