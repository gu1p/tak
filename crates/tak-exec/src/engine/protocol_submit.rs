/// Submits one remote attempt after successful preflight.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_submit(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
    include_workspace_archive: bool,
) -> Result<RemoteSubmitAck> {
    let body = build_remote_submit_payload(
        target,
        task_run_id,
        attempt,
        task_label,
        task,
        remote_workspace,
        include_workspace_archive,
    )?
    .to_string();

    let (status, response_body) = remote_protocol_http_request(
        target,
        "POST",
        "/v1/tasks/submit",
        Some(&body),
        "submit",
        Duration::from_secs(1),
    )
    .await?;

    if status == 401 || status == 403 {
        bail!(
            "infra error: remote node {} auth failed during submit with HTTP {}",
            target.node_id,
            status
        );
    }
    if status != 200 {
        bail!(
            "infra error: remote node {} submit failed with HTTP {}",
            target.node_id,
            status
        );
    }

    let parsed = serde_json::from_str::<serde_json::Value>(&response_body).ok();
    let accepted = parsed
        .as_ref()
        .and_then(|value| value.get("accepted").and_then(serde_json::Value::as_bool))
        .unwrap_or(true);
    if !accepted {
        let is_auth_rejection = parsed
            .as_ref()
            .and_then(|value| value.get("reason").and_then(serde_json::Value::as_str))
            .map(|reason| reason.eq_ignore_ascii_case("auth_failed"))
            .unwrap_or(false);
        if is_auth_rejection {
            bail!(
                "infra error: remote node {} auth failed during submit",
                target.node_id
            );
        }
        bail!(
            "infra error: remote node {} rejected submit for task {} attempt {}",
            target.node_id,
            task_label,
            attempt
        );
    }

    let remote_worker = parsed
        .as_ref()
        .and_then(|value| {
            value
                .get("execution_mode")
                .and_then(serde_json::Value::as_str)
        })
        .map(|mode| mode == "remote_worker")
        .or_else(|| {
            parsed.as_ref().and_then(|value| {
                value
                    .get("remote_worker")
                    .and_then(serde_json::Value::as_bool)
            })
        })
        .unwrap_or(false);

    Ok(RemoteSubmitAck { remote_worker })
}
