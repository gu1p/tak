use tak_core::model::RemoteSelectionSpec;

use super::{PlacementMode, TaskRunResult};

use super::attempt_execution::AttemptExecutionOutcome;
use super::remote_models::{RuntimeExecutionMetadata, TaskPlacement};
use super::session_workspaces::PreparedTaskSession;

pub(crate) struct TaskRunResultContext<'a> {
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) success: bool,
    pub(crate) placement: &'a TaskPlacement,
    /// Paths-only manifest hash of the task's context. Threaded explicitly (rather than read
    /// from the staged workspace) so it is preserved even when staging is skipped on a per-job
    /// upload-cache hit.
    pub(crate) context_manifest_hash: Option<String>,
    pub(crate) runtime_metadata: Option<&'a RuntimeExecutionMetadata>,
    pub(crate) session: Option<&'a PreparedTaskSession>,
}

pub(crate) fn build_task_run_result(
    context: TaskRunResultContext<'_>,
    outcome: AttemptExecutionOutcome,
) -> TaskRunResult {
    TaskRunResult {
        task_run_id: context.task_run_id.to_string(),
        attempts: context.attempt,
        success: context.success,
        exit_code: outcome.last_exit_code,
        failure_detail: outcome.failure_detail,
        placement_mode: context.placement.placement_mode,
        remote_node_id: context.placement.remote_node_id.clone(),
        remote_transport_kind: context
            .placement
            .strict_remote_target
            .as_ref()
            .map(|target| target.transport_kind.as_result_value().to_string()),
        decision_reason: context.placement.decision_reason.clone(),
        context_manifest_hash: context.context_manifest_hash,
        remote_runtime_kind: outcome
            .remote_runtime_kind
            .or_else(|| context.runtime_metadata.map(|meta| meta.kind.clone())),
        remote_runtime_engine: outcome.remote_runtime_engine.or_else(|| {
            context
                .runtime_metadata
                .and_then(|meta| meta.engine.clone())
        }),
        session_name: context.session.map(|session| session.display_name.clone()),
        session_reuse: context
            .session
            .map(|session| session.reuse.as_str().to_string()),
        remote_logs: outcome.remote_logs,
        synced_outputs: outcome.synced_outputs,
    }
}

pub(crate) fn empty_task_result() -> TaskRunResult {
    let placement = TaskPlacement {
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
        strict_remote_target: None,
        ordered_remote_targets: Vec::new(),
        remote_selection: RemoteSelectionSpec::Sequential,
        decision_reason: None,
        local: None,
        remote: None,
        session: None,
    };
    build_task_run_result(
        TaskRunResultContext {
            task_run_id: "",
            attempt: 1,
            success: true,
            placement: &placement,
            context_manifest_hash: None,
            runtime_metadata: None,
            session: None,
        },
        AttemptExecutionOutcome {
            attempt_success: true,
            last_exit_code: Some(0),
            failure_detail: None,
            synced_outputs: Vec::new(),
            remote_runtime_kind: None,
            remote_runtime_engine: None,
            remote_logs: Vec::new(),
        },
    )
}
