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
) -> Result<StrictRemoteTarget> {
    let mut failures = Vec::new();

    for candidate in candidates {
        match preflight_strict_remote_target(candidate).await {
            Ok(()) => return Ok(candidate.clone()),
            Err(err) => failures.push(err.to_string()),
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
) -> Result<StrictRemoteTarget> {
    let mut failures = vec![initial_failure];

    for candidate in candidates {
        if candidate.node_id == failed_node_id {
            continue;
        }

        match preflight_strict_remote_target(candidate).await {
            Ok(()) => {}
            Err(err) => {
                failures.push(err.to_string());
                continue;
            }
        }

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
            Ok(()) => return Ok(candidate.clone()),
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
