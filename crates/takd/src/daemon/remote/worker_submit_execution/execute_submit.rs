fn execute_remote_worker_submit(
    context: RemoteWorkerSubmitRunContext<'_>,
) -> Result<(RemoteWorkerExecutionOutcome, Vec<RemoteWorkerOutputRecord>)> {
    let RemoteWorkerSubmitRunContext {
        idempotency_key,
        execution_root_base,
        selected_node_id,
        image_cache,
        payload,
        output_observer,
        cancellation,
        status_context,
    } = context;
    let execution_root = execution_root_for_payload(idempotency_key, execution_root_base, payload)?;
    let artifact_root = artifact_root_for_submit_key_at_base(idempotency_key, execution_root_base);
    prepare_execution_root(&execution_root, payload)?;
    refresh_session_storage_parent_for_submit(idempotency_key, execution_root_base, payload);

    let execution_result = (|| -> Result<_> {
        unpack_payload_workspace(payload, &execution_root)?;
        overlay_session_paths(execution_root_base, payload, &execution_root)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create tokio runtime for remote worker execution")?;
        let result = runtime.block_on(execute_payload_steps(PayloadStepsContext {
            execution_root: &execution_root,
            idempotency_key,
            selected_node_id,
            image_cache,
            payload,
            output_observer: output_observer.clone(),
            cancellation,
            status_context: status_context.clone(),
        }))?;
        let success = result.result().success;
        let outputs =
            collect_declared_remote_worker_outputs(&execution_root, &payload.outputs, success)?;
        stage_remote_worker_outputs(&artifact_root, &execution_root, &outputs)?;
        if success {
            persist_session_paths(execution_root_base, payload, &execution_root)?;
        }

        Ok((result, outputs))
    })();

    refresh_session_storage_parent_for_submit(idempotency_key, execution_root_base, payload);
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
    input: PayloadStepsContext<'_>,
) -> Result<RemoteWorkerExecutionOutcome> {
    let PayloadStepsContext {
        execution_root,
        idempotency_key,
        selected_node_id,
        image_cache,
        payload,
        output_observer,
        cancellation,
        status_context,
    } = input;
    let context = RemoteMemberExecutionContext {
        execution_root,
        submit_key: idempotency_key,
        task_run_id: payload.task_run_id.clone(),
        selected_node_id,
        image_cache,
        runtime: payload.runtime.clone(),
        output_observer,
        cancellation,
        status_context,
    };
    if payload.fused_members.is_empty() {
        return execute_one_remote_member(
            &context,
            &payload.task_label,
            payload.execution_label.as_deref(),
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
) -> Result<RemoteWorkerExecutionOutcome> {
    let mut last_result = None;
    for member in &payload.fused_members {
        let result = execute_remote_member_with_retries(context, member).await?;
        let success = result.result().success;
        last_result = Some(result);
        if !success {
            return Ok(last_result.expect("fused member result was just stored"));
        }
    }
    Ok(last_result.expect("fused payload contains at least one member"))
}

async fn execute_remote_member_with_retries(
    context: &RemoteMemberExecutionContext<'_>,
    member: &RemoteWorkerFusedMember,
) -> Result<RemoteWorkerExecutionOutcome> {
    let mut member_attempt = 0;
    loop {
        member_attempt += 1;
        let result = execute_one_remote_member(
            context,
            &member.task_label,
            member.execution_label.as_deref(),
            member_attempt,
            member.steps.clone(),
            member.timeout_s,
        )
        .await?;
        if result.result().success
            || result.container_oom_killed() == Some(true)
            || !can_retry(member, member_attempt, result.result().exit_code)
        {
            return Ok(result);
        }
        wait_before_retry(member, member_attempt).await;
    }
}

async fn execute_one_remote_member(
    context: &RemoteMemberExecutionContext<'_>,
    task_label: &str,
    execution_label: Option<&str>,
    attempt: u32,
    steps: Vec<StepDef>,
    timeout_s: Option<u64>,
) -> Result<RemoteWorkerExecutionOutcome> {
    update_active_member_status(context, task_label, execution_label);
    let task_label = parse_label(task_label, "//")
        .map_err(|err| anyhow!("invalid submit task label {task_label}: {err}"))?;
    execute_remote_worker_steps_with_output_and_cancellation(
        context.execution_root,
        &RemoteWorkerExecutionSpec {
            task_label,
            task_run_id: context.task_run_id.clone(),
            attempt,
            steps,
            base_environment: Default::default(),
            clear_environment: false,
            timeout_s,
            runtime: context.runtime.clone(),
            node_id: context.selected_node_id.to_string(),
            container_user: remote_container_user(),
            image_cache: context.image_cache.map(image_cache_options),
            container_identity: Some(tak_runner::ContainerExecutionIdentity {
                owner: super::container_ownership::OWNER_VALUE.to_string(),
                submit_key: context.submit_key.to_string(),
                task_run_id: context.task_run_id.clone(),
            }),
        },
        Some(context.output_observer.clone()),
        context.cancellation,
    )
    .await
}
