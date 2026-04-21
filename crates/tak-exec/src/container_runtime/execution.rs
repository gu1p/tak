use super::*;

pub(super) async fn run_step_in_container(
    docker: &Docker,
    engine: ContainerEngine,
    podman_wait_socket: Option<&str>,
    image: &str,
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
        image: Some(image.to_string()),
        cmd: Some(step.argv.clone()),
        env: Some(env),
        working_dir: Some(step.cwd.to_string_lossy().to_string()),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(false),
        host_config: Some(HostConfig {
            binds: Some(vec![bind_mount]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let created = docker
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
    docker
        .start_container(&container_id, None::<StartContainerOptions<String>>)
        .await
        .context("infra error: container lifecycle start failed: start container failed")?;
    let log_task = spawn_container_log_task(
        docker.clone(),
        container_id.clone(),
        run_context.task_label.clone(),
        run_context.attempt,
        run_context.output_observer.cloned(),
    );

    let wait_result =
        wait_for_container_exit_code(docker, engine, podman_wait_socket, &container_id);
    let status = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), wait_result).await {
            Ok(result) => result?,
            Err(_) => {
                let _ = cleanup_container(docker, &container_id).await;
                let _ = finish_container_log_task(log_task).await;
                return Ok(StepRunResult {
                    success: false,
                    exit_code: None,
                });
            }
        }
    } else {
        wait_result.await?
    };
    finish_container_log_task(log_task).await?;

    let _ = cleanup_container(docker, &container_id).await;
    Ok(StepRunResult {
        success: status == 0,
        exit_code: Some(status),
    })
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
    let _ = docker
        .remove_container(
            container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    Ok(())
}
