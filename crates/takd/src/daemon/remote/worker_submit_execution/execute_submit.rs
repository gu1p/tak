fn execute_remote_worker_submit(
    idempotency_key: &str,
    execution_root_base: &Path,
    selected_node_id: &str,
    image_cache: Option<&super::types::RemoteImageCacheRuntimeConfig>,
    payload: &RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &tak_runner::RunCancellation,
) -> Result<(
    tak_runner::RemoteWorkerExecutionResult,
    Vec<RemoteWorkerOutputRecord>,
)> {
    let execution_root =
        execution_root_for_payload(idempotency_key, execution_root_base, payload)?;
    let artifact_root = artifact_root_for_submit_key_at_base(idempotency_key, execution_root_base);
    prepare_execution_root(&execution_root, payload)?;

    let execution_result = (|| -> Result<_> {
        unpack_payload_workspace(payload, &execution_root)?;
        overlay_session_paths(execution_root_base, payload, &execution_root)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create tokio runtime for remote worker execution")?;
        let result = runtime.block_on(execute_payload_steps(
            &execution_root,
            selected_node_id,
            image_cache,
            payload,
            output_observer.clone(),
            cancellation,
        ))?;
        let outputs = collect_declared_remote_worker_outputs(
            &execution_root,
            &payload.outputs,
            result.success,
        )?;
        stage_remote_worker_outputs(&artifact_root, &execution_root, &outputs)?;
        if result.success {
            persist_session_paths(execution_root_base, payload, &execution_root)?;
        }

        Ok((result, outputs))
    })();

    let cleanup_result = cleanup_execution_root(payload, &execution_root);

    match (execution_result, cleanup_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(value), Err(err)) => {
            tracing::warn!(
                "remote worker submit {idempotency_key} completed successfully but cleanup failed: {err:#}"
            );
            Ok(value)
        }
        (Err(err), Ok(())) => Err(err),
        (Err(err), Err(cleanup_err)) => Err(err.context(cleanup_err.to_string())),
    }
}

async fn execute_payload_steps(
    execution_root: &Path,
    selected_node_id: &str,
    image_cache: Option<&super::types::RemoteImageCacheRuntimeConfig>,
    payload: &RemoteWorkerSubmitPayload,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &tak_runner::RunCancellation,
) -> Result<tak_runner::RemoteWorkerExecutionResult> {
    let context = RemoteMemberExecutionContext {
        execution_root,
        selected_node_id,
        image_cache,
        runtime: payload.runtime.clone(),
        output_observer,
        cancellation,
    };
    if payload.fused_members.is_empty() {
        return execute_one_remote_member(
            &context,
            &payload.task_label,
            payload.attempt,
            payload.steps.clone(),
            payload.timeout_s,
        )
        .await;
    }
    execute_fused_remote_members(&context, payload).await
}

async fn execute_fused_remote_members(
    context: &RemoteMemberExecutionContext<'_>,
    payload: &RemoteWorkerSubmitPayload,
) -> Result<tak_runner::RemoteWorkerExecutionResult> {
    let mut last_result = successful_remote_worker_result();
    for member in &payload.fused_members {
        let result = execute_remote_member_with_retries(context, member).await?;
        let success = result.success;
        last_result = result;
        if !success {
            return Ok(last_result);
        }
    }
    Ok(last_result)
}

struct RemoteMemberExecutionContext<'a> {
    execution_root: &'a Path,
    selected_node_id: &'a str,
    image_cache: Option<&'a super::types::RemoteImageCacheRuntimeConfig>,
    runtime: Option<RemoteRuntimeSpec>,
    output_observer: Arc<dyn TaskOutputObserver>,
    cancellation: &'a tak_runner::RunCancellation,
}

async fn execute_remote_member_with_retries(
    context: &RemoteMemberExecutionContext<'_>,
    member: &RemoteWorkerFusedMember,
) -> Result<tak_runner::RemoteWorkerExecutionResult> {
    let mut member_attempt = 0;
    loop {
        member_attempt += 1;
        let result = execute_one_remote_member(
            context,
            &member.task_label,
            member_attempt,
            member.steps.clone(),
            member.timeout_s,
        )
        .await?;
        if result.success || !can_retry(member, member_attempt, result.exit_code) {
            return Ok(result);
        }
        wait_before_retry(member, member_attempt).await;
    }
}

async fn execute_one_remote_member(
    context: &RemoteMemberExecutionContext<'_>,
    task_label: &str,
    attempt: u32,
    steps: Vec<StepDef>,
    timeout_s: Option<u64>,
) -> Result<tak_runner::RemoteWorkerExecutionResult> {
    let task_label = parse_label(task_label, "//")
        .map_err(|err| anyhow!("invalid submit task label {task_label}: {err}"))?;
    execute_remote_worker_steps_with_output_and_cancellation(
        context.execution_root,
        &RemoteWorkerExecutionSpec {
            task_label,
            attempt,
            steps,
            timeout_s,
            runtime: context.runtime.clone(),
            node_id: context.selected_node_id.to_string(),
            container_user: remote_container_user(),
            image_cache: context.image_cache.map(image_cache_options),
        },
        Some(context.output_observer.clone()),
        context.cancellation,
    )
    .await
}

fn successful_remote_worker_result() -> tak_runner::RemoteWorkerExecutionResult {
    tak_runner::RemoteWorkerExecutionResult {
        success: true,
        exit_code: Some(0),
        runtime_kind: None,
        runtime_engine: None,
    }
}
