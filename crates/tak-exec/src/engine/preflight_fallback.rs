use anyhow::{Context, Result, bail};
use tak_core::model::ResolvedTask;

use super::{
    RemotePreflightExhaustedError, RemotePreflightFailure, TaskOutputObserver,
    preflight_failure::{
        RemoteNodeInfoFailure, remote_preflight_error_failure, remote_preflight_timeout_failure,
        remote_preflight_unhealthy_failure,
    },
    preflight_status_output::{
        emit_remote_accepted, emit_remote_connected, emit_remote_probe, emit_remote_submit,
        emit_remote_unavailable, next_candidate_available,
    },
    protocol_detection::detect_remote_protocol_mode,
    protocol_submit::remote_protocol_submit,
    remote_models::{RemoteSubmitContext, StrictRemoteTarget},
    remote_submit_failure::{RemoteSubmitFailure, RemoteSubmitFailureKind},
    transport,
};
use crate::client_observations::record_remote_observation;

pub(crate) async fn preflight_ordered_remote_target(
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
                failures.push(err);
                if index + 1 < candidates.len() {
                    emit_remote_unavailable(output_observer, &task.label, 1, &candidate.node_id)?;
                }
            }
        }
    }

    Err(RemotePreflightExhaustedError {
        task_label: task.label.to_string(),
        failures,
    }
    .into())
}

pub(crate) async fn preflight_strict_remote_target(
    target: &StrictRemoteTarget,
) -> std::result::Result<(), RemotePreflightFailure> {
    if let Err(err) = transport::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    }) {
        return Err(remote_preflight_error_failure(
            target,
            RemoteNodeInfoFailure::other(format!("{err:#}")),
        ));
    }

    let preflight_timeout = transport::preflight_timeout(target);
    match tokio::time::timeout(preflight_timeout, detect_remote_protocol_mode(target)).await {
        Ok(Ok(node)) => {
            let _ = record_remote_observation(&node);
            if node.healthy {
                Ok(())
            } else {
                Err(remote_preflight_unhealthy_failure(target, &node))
            }
        }
        Ok(Err(err)) => Err(remote_preflight_error_failure(target, err)),
        Err(_) => Err(remote_preflight_timeout_failure(
            target,
            format!(
                "infra error: remote node {} at {} via {} node info request timed out",
                target.node_id,
                target.endpoint,
                target.transport_kind.as_result_value()
            ),
        )),
    }
}

pub(crate) fn is_auth_submit_failure(err: &anyhow::Error) -> bool {
    err.downcast_ref::<RemoteSubmitFailure>()
        .is_some_and(|failure| failure.kind == RemoteSubmitFailureKind::Auth)
}

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
        )?;
        match remote_protocol_submit(
            candidate,
            submit.task_run_id,
            submit.attempt,
            submit.task_label,
            task,
            submit.remote_workspace,
            submit.session,
        )
        .await
        {
            Ok(()) => {
                emit_remote_accepted(
                    output_observer,
                    &task.label,
                    submit.attempt,
                    &candidate.node_id,
                )?;
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
