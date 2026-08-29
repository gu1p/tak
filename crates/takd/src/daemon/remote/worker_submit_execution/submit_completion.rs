fn finish_remote_worker_submit(
    context: &RemoteNodeContext,
    idempotency_key: &str,
    resolved_execution_root_base: Option<&Path>,
    session: Option<&RemoteWorkerSession>,
) {
    if let Err(error) = context.finish_active_job(idempotency_key) {
        tracing::error!(
            "failed to clear active node status entry for submit {idempotency_key}: {error:#}"
        );
    }
    let unregister_result =
        context.unregister_active_execution_after_locked(idempotency_key, || {
            match resolved_execution_root_base {
                Some(root) => refresh_session_storage_parent(root, session),
                None => Ok(()),
            }
        });
    if let Err(error) = unregister_result {
        tracing::error!(
            "failed to refresh session storage while unregistering submit {idempotency_key}: {error:#}"
        );
    }
    if let Err(error) = context.release_resources(idempotency_key) {
        tracing::error!("failed to release resources for submit {idempotency_key}: {error:#}");
    }
}
