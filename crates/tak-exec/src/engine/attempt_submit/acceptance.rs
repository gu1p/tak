use anyhow::Result;
use tak_core::model::ResolvedTask;

use super::AttemptSubmitState;
use crate::engine::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use crate::engine::remote_models::{StrictRemoteTarget, TaskPlacement};
use crate::engine::{TaskOutputObserver, TaskStatusEventKind, TaskStatusPhase};

pub(super) fn record_accepted_target(
    task: &ResolvedTask,
    placement: &mut TaskPlacement,
    submit: &AttemptSubmitState<'_>,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    selected_target: StrictRemoteTarget,
) -> Result<()> {
    placement.remote_node_id = Some(selected_target.node_id.clone());
    placement.strict_remote_target = Some(selected_target.clone());
    let accepted_message = if selected_target.daemon_task_handle.is_some() {
        format!(
            "remote worker {} selected by local takd; task accepted",
            selected_target.node_id
        )
    } else {
        format!("remote task accepted by {}", selected_target.node_id)
    };
    emit_task_status_message_with_details(
        output_observer,
        &task.label,
        submit.attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(selected_target.node_id.as_str()),
        accepted_message,
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::WorkerSelected),
            transport: Some(selected_target.transport_kind.as_result_value().to_string()),
            ..TaskStatusDetails::default()
        },
    )
}
