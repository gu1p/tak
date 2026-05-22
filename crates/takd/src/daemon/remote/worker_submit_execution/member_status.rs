#[derive(Clone)]
struct RemoteMemberStatusContext {
    context: RemoteNodeContext,
    idempotency_key: String,
}

fn update_active_member_status(
    context: &RemoteMemberExecutionContext<'_>,
    task_label: &str,
    execution_label: Option<&str>,
) {
    let Some(status) = &context.status_context else {
        return;
    };
    if let Err(error) = status.context.update_active_job_label(
        &status.idempotency_key,
        task_label,
        execution_label.map(str::to_string),
    ) {
        tracing::warn!(
            "failed to update active job label for submit {}: {error:#}",
            status.idempotency_key
        );
    }
}
