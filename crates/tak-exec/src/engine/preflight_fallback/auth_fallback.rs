pub(crate) async fn fallback_after_auth_submit_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    submit: RemoteSubmitContext<'_>,
    initial_failure: String,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StrictRemoteTarget> {
    let mut failures = vec![initial_failure.clone()];
    let mut preflight_failures = Vec::new();
    if candidates
        .iter()
        .any(|candidate| candidate.node_id != failed_node_id)
    {
        emit_remote_unavailable(output_observer, &task.label, submit.attempt, failed_node_id)?;
    }
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.node_id == failed_node_id {
            continue;
        }

        emit_remote_probe(
            output_observer,
            &task.label,
            submit.attempt,
            &candidate.node_id,
        )?;
        match preflight_strict_remote_target(candidate).await {
            Ok(()) => emit_remote_connected(
                output_observer,
                &task.label,
                submit.attempt,
                &candidate.node_id,
            )?,
            Err(err) => {
                failures.push(err.message.clone());
                preflight_failures.push(err);
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

        emit_remote_submit(
            output_observer,
            &task.label,
            submit.attempt,
            &candidate.node_id,
            &submit.remote_workspace.upload_size_mb(),
        )?;
        match remote_protocol_submit(RemoteProtocolSubmit {
            target: candidate,
            task_run_id: submit.task_run_id,
            attempt: submit.attempt,
            task,
            remote_workspace: submit.remote_workspace,
            session: submit.session,
            fused_members: submit.fused_members,
            execution_label: submit.execution_label,
            fused_member_execution_labels: submit.fused_member_execution_labels,
            output_observer,
        })
        .await
        {
            Ok(selected_target) => {
                emit_remote_accepted(
                    output_observer,
                    &task.label,
                    submit.attempt,
                    &selected_target.node_id,
                )?;
                return Ok(selected_target);
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
            }
        }
    }

    if !preflight_failures.is_empty() && failures.len() == preflight_failures.len() + 1 {
        let exhausted: Result<StrictRemoteTarget> = Err(RemotePreflightExhaustedError {
            task_label: task.label.to_string(),
            failures: preflight_failures,
        }
        .into());
        return exhausted.context(initial_failure);
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}
