use anyhow::{Context, Result, bail};
use tak_core::model::ResolvedTask;

use super::{
    RemotePreflightExhaustedError, RemotePreflightFailure, TaskOutputObserver,
    preflight_capacity::remote_target_has_capacity,
    preflight_failure::{
        RemoteNodeInfoFailure, remote_preflight_error_failure, remote_preflight_timeout_failure,
        remote_preflight_unhealthy_failure,
    },
    preflight_status_output::{
        emit_remote_accepted, emit_remote_connected, emit_remote_probe, emit_remote_submit,
        emit_remote_unavailable, next_candidate_available,
    },
    protocol_detection::detect_remote_protocol_mode,
    protocol_submit::{RemoteProtocolSubmit, remote_protocol_submit},
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

    let mut reachable = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        emit_remote_probe(output_observer, &task.label, 1, &candidate.node_id)?;
        match preflight_strict_remote_target(candidate).await {
            Ok(()) => {
                emit_remote_connected(output_observer, &task.label, 1, &candidate.node_id)?;
                reachable.push(candidate.clone());
            }
            Err(err) => {
                failures.push(err);
                if index + 1 < candidates.len() {
                    emit_remote_unavailable(output_observer, &task.label, 1, &candidate.node_id)?;
                }
            }
        }
    }

    for candidate in &reachable {
        if remote_target_has_capacity(candidate).await.unwrap_or(true) {
            return Ok(candidate.clone());
        }
    }
    if let Some(candidate) = reachable.into_iter().next() {
        return Ok(candidate);
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

include!("preflight_fallback/auth_fallback.rs");
