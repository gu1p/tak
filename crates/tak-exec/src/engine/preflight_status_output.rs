use super::*;

pub(crate) fn emit_remote_probe(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("probing remote node {node_id}"),
    )
}

pub(crate) fn emit_remote_connected(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("connected to remote node {node_id}"),
    )
}

pub(crate) fn emit_remote_unavailable(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteProbe,
        Some(node_id),
        format!("remote node {node_id} unavailable, trying next candidate"),
    )
}

pub(crate) fn emit_remote_submit(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(node_id),
        format!("submitting to remote node {node_id}"),
    )
}

pub(crate) fn emit_remote_accepted(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteSubmit,
        Some(node_id),
        format!("remote task accepted by {node_id}"),
    )
}

pub(crate) fn next_candidate_available(
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    index: usize,
) -> bool {
    candidates[index + 1..]
        .iter()
        .any(|next| next.node_id != failed_node_id)
}
