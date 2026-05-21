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

async fn wait_for_container_step(
    docker: &Docker,
    engine: ContainerEngine,
    podman_wait_socket: Option<&str>,
    container_id: &str,
    timeout_s: Option<u64>,
    cancellation: &RunCancellation,
) -> Result<Option<i32>> {
    let wait = wait_for_container_exit_code(docker, engine, podman_wait_socket, container_id);
    tokio::pin!(wait);
    if let Some(seconds) = timeout_s {
        let timeout = tokio::time::sleep(Duration::from_secs(seconds));
        tokio::pin!(timeout);
        return tokio::select! {
            result = &mut wait => Ok(Some(result?)),
            _ = &mut timeout => Ok(None),
            _ = cancellation.cancelled() => Err(crate::engine::cancelled_error()),
        };
    }
    tokio::select! {
        result = &mut wait => Ok(Some(result?)),
        _ = cancellation.cancelled() => Err(crate::engine::cancelled_error()),
    }
}

async fn wait_for_container_exit_code(
    docker: &Docker,
    engine: ContainerEngine,
    podman_wait_socket: Option<&str>,
    container_name: &str,
) -> Result<i32> {
    match engine {
        ContainerEngine::Docker => {
            wait_for_container_exit_code_via_api(docker, container_name).await
        }
        ContainerEngine::Podman => {
            wait_for_container_exit_code_via_cli(podman_wait_socket, container_name).await
        }
    }
}

async fn wait_for_container_exit_code_via_api(
    docker: &Docker,
    container_name: &str,
) -> Result<i32> {
    let mut stream = docker.wait_container(container_name, None::<WaitContainerOptions<String>>);
    let Some(result) = stream.next().await else {
        bail!("infra error: container lifecycle runtime failed: wait stream ended unexpectedly");
    };
    docker_wait_result_exit_code(result)
}

pub(super) fn docker_wait_result_exit_code(
    result: std::result::Result<bollard::models::ContainerWaitResponse, BollardError>,
) -> Result<i32> {
    match result {
        Ok(result) => docker_wait_exit_code(result.status_code),
        Err(BollardError::DockerContainerWaitError { error, code }) if error.is_empty() => {
            docker_wait_exit_code(code)
        }
        Err(BollardError::DockerContainerWaitError { error, code }) => {
            bail!(
                "infra error: container lifecycle runtime failed: docker wait failed (status {code}): {error}"
            );
        }
        Err(err) => Err(err).context(
            "infra error: container lifecycle runtime failed: waiting for container failed",
        ),
    }
}

fn docker_wait_exit_code(code: i64) -> Result<i32> {
    i32::try_from(code)
        .context("infra error: container lifecycle runtime failed: invalid docker wait exit code")
}

async fn wait_for_container_exit_code_via_cli(
    podman_wait_socket: Option<&str>,
    container_name: &str,
) -> Result<i32> {
    let podman_wait_socket = podman_wait_socket.map(ToString::to_string);
    let container_name = container_name.to_string();
    let output = tokio::task::spawn_blocking(move || {
        let mut cmd = StdCommand::new("podman");
        if let Some(socket) = podman_wait_socket.as_deref() {
            cmd.args(["--url", socket]);
        }
        cmd.args(["wait", &container_name]).output()
    })
    .await
    .context("infra error: container lifecycle runtime failed: podman wait join failed")?
    .context("infra error: container lifecycle runtime failed: podman wait launch failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("infra error: container lifecycle runtime failed: podman wait failed: {stderr}");
    }

    let value = String::from_utf8_lossy(&output.stdout);
    let exit_code = value
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .parse::<i32>()
        .context(
            "infra error: container lifecycle runtime failed: invalid podman wait exit code",
        )?;
    Ok(exit_code)
}

async fn cleanup_container(docker: &Docker, container_name: &str) -> Result<()> {
    let mut last_error = None;
    for attempt in 0..3 {
        match docker
            .remove_container(
                container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
        {
            Ok(()) => return Ok(()),
            Err(err) if container_was_already_removed(&err) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
        if attempt < 2 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    let Some(error) = last_error else {
        return Ok(());
    };
    Err(error).with_context(|| {
        format!(
            "infra error: container lifecycle cleanup failed: remove container {container_name} failed"
        )
    })
}

fn container_was_already_removed(error: &BollardError) -> bool {
    matches!(
        error,
        BollardError::DockerResponseServerError {
            status_code: 404,
            ..
        }
    )
}
