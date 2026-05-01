use tak_core::model::RemoteSelectionSpec;

use super::{PlacementMode, RemoteWorkspaceStage, TaskRunResult};

use super::attempt_execution::AttemptExecutionOutcome;
use super::remote_models::{RuntimeExecutionMetadata, TaskPlacement};
use super::session_workspaces::PreparedTaskSession;

pub(crate) fn build_task_run_result(
    attempt: u32,
    success: bool,
    placement: &TaskPlacement,
    remote_workspace: Option<&RemoteWorkspaceStage>,
    runtime_metadata: Option<&RuntimeExecutionMetadata>,
    session: Option<&PreparedTaskSession>,
    outcome: AttemptExecutionOutcome,
) -> TaskRunResult {
    TaskRunResult {
        attempts: attempt,
        success,
        exit_code: outcome.last_exit_code,
        failure_detail: outcome.failure_detail,
        placement_mode: placement.placement_mode,
        remote_node_id: placement.remote_node_id.clone(),
        remote_transport_kind: placement
            .strict_remote_target
            .as_ref()
            .map(|target| target.transport_kind.as_result_value().to_string()),
        decision_reason: placement.decision_reason.clone(),
        context_manifest_hash: remote_workspace.map(|staged| staged.manifest_hash.clone()),
        remote_runtime_kind: outcome
            .remote_runtime_kind
            .or_else(|| runtime_metadata.map(|meta| meta.kind.clone())),
        remote_runtime_engine: outcome
            .remote_runtime_engine
            .or_else(|| runtime_metadata.and_then(|meta| meta.engine.clone())),
        session_name: session.map(|session| session.display_name.clone()),
        session_reuse: session.map(|session| session.reuse.as_str().to_string()),
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
        1,
        true,
        &placement,
        None,
        None,
        None,
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
