use std::path::PathBuf;

use tak_proto::SubmittedNeed;

#[derive(Clone)]
pub(crate) struct ActiveJobMetadata {
    pub(crate) task_run_id: String,
    pub(crate) attempt: u32,
    pub(crate) task_label: String,
    pub(crate) started_at_ms: i64,
    pub(crate) needs: Vec<SubmittedNeed>,
    pub(crate) runtime: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) runtime_source: Option<String>,
    pub(crate) command: Option<String>,
    pub(crate) resource_limits: Option<tak_core::model::ContainerResourceLimitsSpec>,
    pub(crate) execution_label: Option<String>,
    pub(crate) execution_root: PathBuf,
}

pub(crate) struct ActiveJobMetadataInput<'a> {
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) task_label: &'a str,
    pub(crate) started_at_ms: i64,
    pub(crate) needs: &'a [SubmittedNeed],
    pub(crate) runtime: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) runtime_source: Option<String>,
    pub(crate) command: Option<String>,
    pub(crate) resource_limits: Option<tak_core::model::ContainerResourceLimitsSpec>,
    pub(crate) execution_label: Option<String>,
    pub(crate) execution_root: PathBuf,
}

impl ActiveJobMetadata {
    pub(crate) fn new(input: ActiveJobMetadataInput<'_>) -> Self {
        Self {
            task_run_id: input.task_run_id.to_string(),
            attempt: input.attempt,
            task_label: input.task_label.to_string(),
            started_at_ms: input.started_at_ms,
            needs: input.needs.to_vec(),
            runtime: input.runtime,
            origin: input.origin,
            runtime_source: input.runtime_source,
            command: input.command,
            resource_limits: input.resource_limits,
            execution_label: input.execution_label,
            execution_root: input.execution_root,
        }
    }
}
