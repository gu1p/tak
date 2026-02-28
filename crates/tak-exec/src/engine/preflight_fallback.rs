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
) -> Result<(StrictRemoteTarget, RemoteProtocolMode)> {
    let mut failures = Vec::new();

    for candidate in candidates {
        match preflight_strict_remote_target(candidate).await {
            Ok(mode) => {
                if should_reject_legacy_remote_mode(task, candidate, mode) {
                    failures.push(legacy_protocol_error_message(candidate));
                    continue;
                }
                return Ok((candidate.clone(), mode));
            }
            Err(err) => failures.push(err.to_string()),
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}

fn should_reject_legacy_remote_mode(
    task: &ResolvedTask,
    target: &StrictRemoteTarget,
    mode: RemoteProtocolMode,
) -> bool {
    matches!(mode, RemoteProtocolMode::LegacyReachability)
        && matches!(task.execution, TaskExecutionSpec::RemoteOnly(_))
        && target.runtime.is_none()
}

fn legacy_protocol_error_message(target: &StrictRemoteTarget) -> String {
    format!(
        "infra error: remote node {} at {} does not support V1 handshake protocol",
        target.node_id, target.endpoint
    )
}

/// Performs strict remote preflight by checking endpoint reachability before task execution.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn preflight_strict_remote_target(target: &StrictRemoteTarget) -> Result<RemoteProtocolMode> {
    TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;

    let preflight_timeout = TransportFactory::preflight_timeout(target);
    match tokio::time::timeout(preflight_timeout, TransportFactory::connect(target)).await {
        Ok(Ok(stream)) => {
            drop(stream);
            detect_remote_protocol_mode(target).await
        }
        Ok(Err(err)) => bail!(
            "infra error: remote node {} unavailable at {}: {err}",
            target.node_id,
            target.endpoint
        ),
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

fn is_auth_configuration_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("service auth token")
}

fn is_container_lifecycle_failure(err: &anyhow::Error) -> bool {
    format!("{err:#}").contains("container lifecycle")
}

async fn fallback_after_container_lifecycle_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    initial_failure: String,
) -> Result<(
    StrictRemoteTarget,
    RemoteProtocolMode,
    Option<RuntimeExecutionMetadata>,
)> {
    let mut failures = vec![initial_failure];

    for candidate in candidates {
        if candidate.node_id == failed_node_id {
            continue;
        }

        let mode = match preflight_strict_remote_target(candidate).await {
            Ok(mode) => mode,
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        };

        match resolve_runtime_execution_metadata_for_target(task, candidate) {
            Ok(runtime_metadata) => return Ok((candidate.clone(), mode, runtime_metadata)),
            Err(err) => {
                failures.push(err.to_string());
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

async fn fallback_after_auth_submit_failure(
    task: &ResolvedTask,
    candidates: &[StrictRemoteTarget],
    failed_node_id: &str,
    submit: RemoteSubmitContext<'_>,
    initial_failure: String,
) -> Result<(StrictRemoteTarget, RemoteProtocolMode, RemoteSubmitAck)> {
    let mut failures = vec![initial_failure];

    for candidate in candidates {
        if candidate.node_id == failed_node_id {
            continue;
        }

        let mode = match preflight_strict_remote_target(candidate).await {
            Ok(mode) => mode,
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        };

        if mode.is_handshake_v1() {
            match remote_protocol_submit(
                candidate,
                submit.task_run_id,
                submit.attempt,
                submit.task_label,
                task,
                submit.remote_workspace,
                mode.remote_worker(),
            )
            .await
            {
                Ok(ack) => return Ok((candidate.clone(), mode, ack)),
                Err(err) => {
                    failures.push(err.to_string());
                    continue;
                }
            }
        } else {
            return Ok((
                candidate.clone(),
                mode,
                RemoteSubmitAck {
                    remote_worker: false,
                },
            ));
        }
    }

    bail!(
        "infra error: no reachable remote fallback candidates for task {}: {}",
        task.label,
        failures.join("; ")
    );
}
