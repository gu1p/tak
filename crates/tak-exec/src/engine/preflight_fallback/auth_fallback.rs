pub(crate) async fn fallback_after_auth_submit_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    submit: RemoteSubmitContext<'_>,
    initial_failure: String,
    infrastructure_failures: &mut Vec<RemoteInfrastructureFailure>,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<StrictRemoteTarget> {
    record_infrastructure_failure(
        infrastructure_failures,
        failed_node_id,
        initial_failure.clone(),
    );
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
                let cause = err.failover_cause(&task.label.to_string());
                record_infrastructure_failure(
                    infrastructure_failures,
                    &candidate.node_id,
                    cause.clone(),
                );
                failures.push(cause);
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
            remote_workspace: Some(submit.remote_workspace),
            session: submit.session,
            fused_members: submit.fused_members,
            execution_label: submit.execution_label,
            fused_member_execution_labels: submit.fused_member_execution_labels,
            output_observer,
            upload_cache: submit.upload_cache,
            workspace_content_hash: submit.workspace_content_hash,
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
                let cause = err.to_string();
                record_infrastructure_failure(
                    infrastructure_failures,
                    &candidate.node_id,
                    cause.clone(),
                );
                failures.push(cause);
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

fn record_infrastructure_failure(
    failures: &mut Vec<RemoteInfrastructureFailure>,
    node_id: &str,
    cause: String,
) {
    if failures.iter().any(|failure| failure.node_id == node_id) {
        return;
    }
    failures.push(RemoteInfrastructureFailure {
        node_id: node_id.to_string(),
        cause,
    });
}
