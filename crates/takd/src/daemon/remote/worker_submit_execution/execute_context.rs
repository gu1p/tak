struct RemoteWorkerSubmitRunContext<'a> {
    idempotency_key: &'a str,
    execution_root_base: &'a Path,
    selected_node_id: &'a str,
    image_cache: Option<&'a super::types::RemoteImageCacheRuntimeConfig>,
    payload: &'a RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &'a tak_runner::RunCancellation,
    status_context: Option<RemoteMemberStatusContext>,
}

struct PayloadStepsContext<'a> {
    execution_root: &'a Path,
    idempotency_key: &'a str,
    selected_node_id: &'a str,
    image_cache: Option<&'a super::types::RemoteImageCacheRuntimeConfig>,
    payload: &'a RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &'a tak_runner::RunCancellation,
    status_context: Option<RemoteMemberStatusContext>,
}

struct RemoteMemberExecutionContext<'a> {
    execution_root: &'a Path,
    submit_key: &'a str,
    task_run_id: String,
    selected_node_id: &'a str,
    image_cache: Option<&'a super::types::RemoteImageCacheRuntimeConfig>,
    runtime: Option<RemoteRuntimeSpec>,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &'a tak_runner::RunCancellation,
    status_context: Option<RemoteMemberStatusContext>,
}
