/// Selects the first reachable remote endpoint in declaration order.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn preflight_ordered_remote_target(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StrictRemoteTarget> {
    let mut failures = Vec::new();

    for (index, candidate) in candidates.iter().enumerate() {
        emit_remote_probe(output_observer, &task.label, 1, &candidate.node_id)?;
        match preflight_strict_remote_target(candidate).await {
            Ok(()) => {
                emit_remote_connected(output_observer, &task.label, 1, &candidate.node_id)?;
                return Ok(candidate.clone());
            }
            Err(err) => {
                failures.push(err.to_string());
                if index + 1 < candidates.len() {
                    emit_remote_unavailable(output_observer, &task.label, 1, &candidate.node_id)?;
                }
            }
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

/// Performs strict remote preflight by checking endpoint reachability before task execution.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn preflight_strict_remote_target(target: &StrictRemoteTarget) -> Result<()> {
    TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;

    let preflight_timeout = TransportFactory::preflight_timeout(target);
    match tokio::time::timeout(preflight_timeout, detect_remote_protocol_mode(target)).await {
        Ok(result) => result,
        Err(_) => bail!(
            "infra error: remote node {} unavailable at {}: preflight timed out",
            target.node_id,
            target.endpoint
        ),
    }
}

fn is_auth_submit_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("auth failed")
}

async fn fallback_after_auth_submit_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    submit: RemoteSubmitContext<'_>,
    initial_failure: String,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StrictRemoteTarget> {
    let mut failures = vec![initial_failure];
    if candidates.iter().any(|candidate| candidate.node_id != failed_node_id) {
        emit_remote_unavailable(output_observer, &task.label, submit.attempt, failed_node_id)?;
    }

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.node_id == failed_node_id {
            continue;
        }

        emit_remote_probe(output_observer, &task.label, submit.attempt, &candidate.node_id)?;
        match preflight_strict_remote_target(candidate).await {
            Ok(()) => emit_remote_connected(
                output_observer,
                &task.label,
                submit.attempt,
                &candidate.node_id,
            )?,
            Err(err) => {
                failures.push(err.to_string());
                if next_candidate_available(candidates, failed_node_id, index) {
                    emit_remote_unavailable(
                        output_observer,
                        &task.label,
                        submit.attempt,
                        &candidate.node_id,
                    )?;
                }
                continue;
            }
        }

        emit_remote_submit(output_observer, &task.label, submit.attempt, &candidate.node_id)?;
        match remote_protocol_submit(
            candidate,
            submit.task_run_id,
            submit.attempt,
            submit.task_label,
            task,
            submit.remote_workspace,
        )
        .await
        {
            Ok(()) => {
                emit_remote_accepted(output_observer, &task.label, submit.attempt, &candidate.node_id)?;
                return Ok(candidate.clone());
            }
            Err(err) => {
                failures.push(err.to_string());
                if next_candidate_available(candidates, failed_node_id, index) {
                    emit_remote_unavailable(
                        output_observer,
                        &task.label,
                        submit.attempt,
                        &candidate.node_id,
                    )?;
                }
                continue;
            }
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

fn emit_remote_probe(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(output_observer, task_label, attempt, TaskStatusPhase::RemoteProbe, Some(node_id), format!("probing remote node {node_id}"))
}

fn emit_remote_connected(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(output_observer, task_label, attempt, TaskStatusPhase::RemoteProbe, Some(node_id), format!("connected to remote node {node_id}"))
}

fn emit_remote_unavailable(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(output_observer, task_label, attempt, TaskStatusPhase::RemoteProbe, Some(node_id), format!("remote node {node_id} unavailable, trying next candidate"))
}

fn emit_remote_submit(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(output_observer, task_label, attempt, TaskStatusPhase::RemoteSubmit, Some(node_id), format!("submitting to remote node {node_id}"))
}

fn emit_remote_accepted(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    node_id: &str,
) -> Result<()> {
    emit_task_status_message(output_observer, task_label, attempt, TaskStatusPhase::RemoteSubmit, Some(node_id), format!("remote task accepted by {node_id}"))
}

fn next_candidate_available(
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    index: usize,
) -> bool {
    candidates[index + 1..]
        .iter()
        .any(|next| next.node_id != failed_node_id)
}
