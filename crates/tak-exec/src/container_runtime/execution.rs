use super::*;

pub(super) async fn run_step_in_container(
    executor: &ContainerStepExecutor<'_>,
    step: &ContainerStepSpec,
    timeout_s: Option<u64>,
    run_context: &ContainerStepRunContext<'_>,
) -> Result<StepRunResult> {
    let container_name = format!("tak-step-{}", Uuid::new_v4());
    let bind_mount = format!(
        "{}:{}:rw",
        run_context.workspace_root.display(),
        run_context.workspace_root.display()
    );
    let env = step
        .env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>();

    let config = ContainerConfig {
        image: Some(executor.image.to_string()),
        cmd: Some(step.argv.clone()),
        env: Some(env),
        working_dir: Some(step.cwd.to_string_lossy().to_string()),
        user: run_context.container_user.map(ToString::to_string),
        labels: container_labels(run_context),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        host_config: Some(HostConfig {
            binds: Some(vec![bind_mount]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let created = executor
        .docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            config,
        )
        .await
        .context("infra error: container lifecycle start failed: create container failed")?;
    let container_id = created.id;
    let mut log_task = None;
    let step_result = start_and_wait_for_container_step(
        executor,
        run_context,
        &container_id,
        timeout_s,
        &mut log_task,
    )
    .await;
    let cleanup_result = cleanup_container(executor.docker, &container_id).await;
    let log_result = finish_container_log_task(log_task).await;

    finish_container_step(step_result, cleanup_result, log_result)
}

async fn start_and_wait_for_container_step(
    executor: &ContainerStepExecutor<'_>,
    run_context: &ContainerStepRunContext<'_>,
    container_id: &str,
    timeout_s: Option<u64>,
    log_task: &mut ContainerLogTask,
) -> Result<StepRunResult> {
    executor
        .docker
        .start_container(container_id, None::<StartContainerOptions<String>>)
        .await
        .context("infra error: container lifecycle start failed: start container failed")?;
    *log_task = spawn_container_log_task(
        executor.docker.clone(),
        container_id.to_string(),
        run_context.task_label.clone(),
        run_context.task_run_id.to_string(),
        run_context.attempt,
        run_context.output_observer.cloned(),
    );

    let status = wait_for_container_step(
        executor.docker,
        executor.engine,
        executor.podman_wait_socket,
        container_id,
        timeout_s,
        run_context.cancellation,
    )
    .await;
    let Some(status) = status? else {
        return Ok(StepRunResult {
            success: false,
            exit_code: None,
        });
    };
    if status == 137 {
        emit_exit_137_diagnostic(executor, run_context, container_id).await?;
    }

    Ok(StepRunResult {
        success: status == 0,
        exit_code: Some(status),
    })
}

fn container_labels(run_context: &ContainerStepRunContext<'_>) -> Option<HashMap<String, String>> {
    let identity = run_context.container_identity?;
    Some(HashMap::from([
        ("tak.owner".to_string(), identity.owner.clone()),
        ("tak.submit_key".to_string(), identity.submit_key.clone()),
        ("tak.task_run_id".to_string(), identity.task_run_id.clone()),
        ("tak.attempt".to_string(), run_context.attempt.to_string()),
        (
            "tak.task_label".to_string(),
            run_context.task_label.to_string(),
        ),
    ]))
}

async fn emit_exit_137_diagnostic(
    executor: &ContainerStepExecutor<'_>,
    run_context: &ContainerStepRunContext<'_>,
    container_id: &str,
) -> Result<()> {
    let Some(observer) = run_context.output_observer else {
        return Ok(());
    };
    let oom_state = container_oom_killed(executor.docker, container_id).await;
    observer.observe_status(TaskStatusEvent {
        task_label: run_context.task_label.clone(),
        attempt: run_context.attempt,
        phase: TaskStatusPhase::RemoteWait,
        remote_node_id: None,
        message: exit_137_diagnostic_message(oom_state),
    })
}

async fn container_oom_killed(docker: &Docker, container_id: &str) -> Option<bool> {
    docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|container| container.state)
        .and_then(|state| state.oom_killed)
}

fn exit_137_diagnostic_message(oom_killed: Option<bool>) -> String {
    let oom_state = match oom_killed {
        Some(value) => format!("OOMKilled={value}"),
        None => "OOMKilled=unknown".to_string(),
    };
    format!(
        "container exited with exit code 137 ({oom_state}); Tak resources are reservations only and are not applied as container memory limits"
    )
}

fn finish_container_step(
    step_result: Result<StepRunResult>,
    cleanup_result: Result<()>,
    log_result: Result<()>,
) -> Result<StepRunResult> {
    match (step_result, cleanup_result, log_result) {
        (Ok(result), Ok(()), Ok(())) => Ok(result),
        (Err(err), Err(cleanup_err), _) => Err(err.context(cleanup_err.to_string())),
        (Err(err), _, _) => Err(err),
        (Ok(_), Err(cleanup_err), _) => Err(cleanup_err),
        (Ok(_), Ok(()), Err(log_err)) => Err(log_err),
    }
}
