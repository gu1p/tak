pub(super) fn register_active_job_for_submit(
    execution: &RemoteWorkerSubmitExecution,
    started_at: i64,
) -> Result<()> {
    register_active_job(
        &execution.context,
        &execution.idempotency_key,
        &execution.payload,
        &execution.execution_root_base,
        started_at,
    )
}

pub(super) fn register_active_job(
    context: &RemoteNodeContext,
    idempotency_key: &str,
    payload: &RemoteWorkerSubmitPayload,
    execution_root_base: &std::path::Path,
    started_at: i64,
) -> Result<()> {
    let execution_root =
        execution_root_for_submit_key_at_base(idempotency_key, execution_root_base);
    context.register_active_job(
        idempotency_key.to_string(),
        super::status_state::ActiveJobMetadata::new(super::status_state::ActiveJobMetadataInput {
            task_run_id: &payload.task_run_id,
            attempt: payload.attempt,
            task_label: &payload.task_label,
            started_at_ms: started_at,
            needs: &payload.needs,
            runtime: payload
                .runtime
                .as_ref()
                .map(|_| "containerized".to_string()),
            origin: payload.origin.clone(),
            runtime_source: payload.runtime_source.clone(),
            command: payload.command.clone(),
            resource_limits: runtime_resource_limits(payload.runtime.as_ref()),
            execution_label: payload.execution_label.clone(),
            execution_root,
        }),
    )
}

fn append_queue_event(
    store: &SubmitAttemptStore,
    idempotency_key: &str,
    queue_position: usize,
    timestamp_ms: i64,
) {
    let ahead = queue_position.saturating_sub(1);
    let message = format!("queued on remote node; {ahead} tasks ahead");
    if let Err(error) = store.append_event(
        idempotency_key,
        1,
        &serde_json::json!({
            "kind": "TASK_QUEUED",
            "timestamp_ms": timestamp_ms,
            "queue_position": queue_position,
            "message": message,
        })
        .to_string(),
    ) {
        tracing::error!(
            "failed to append TASK_QUEUED event for submit {idempotency_key}: {error:#}"
        );
    }
}

pub(super) fn runtime_resource_limits(
    runtime: Option<&RemoteRuntimeSpec>,
) -> Option<tak_core::model::ContainerResourceLimitsSpec> {
    match runtime {
        Some(RemoteRuntimeSpec::Containerized {
            resource_limits, ..
        }) => resource_limits.clone(),
        None => None,
    }
}
