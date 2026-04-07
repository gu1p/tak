use tak_proto::SubmitTaskResponse;

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
    _task_label: &str,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
) -> Result<()> {
    let body = build_remote_submit_payload(target, task_run_id, attempt, task, remote_workspace)?
        .encode_to_vec();
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

    let parsed = SubmitTaskResponse::decode(response_body.as_slice()).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for submit",
            target.node_id
        )
    })?;
    if !parsed.accepted {
        bail!(
            "infra error: remote node {} rejected submit for task {} attempt {}",
            target.node_id,
            task.label,
            attempt
        );
    }
    if !parsed.remote_worker {
        bail!(
            "infra error: remote node {} returned submit acknowledgement without remote worker support",
            target.node_id
        );
    }

    Ok(())
}
