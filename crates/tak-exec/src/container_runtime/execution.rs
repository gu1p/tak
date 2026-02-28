async fn run_step_in_container(
    docker: &Docker,
    image: &str,
    step: &ContainerStepSpec,
    timeout_s: Option<u64>,
    workspace_root: &Path,
) -> Result<StepRunResult> {
    let container_name = format!("tak-step-{}", Uuid::new_v4());
    let bind_mount = format!(
        "{}:{}:rw",
        workspace_root.display(),
        workspace_root.display()
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

    docker
        .create_container(
            Some(CreateContainerOptions {
                name: container_name.as_str(),
                platform: None,
            }),
            config,
        )
        .await
        .context("infra error: container lifecycle start failed: create container failed")?;
    docker
        .start_container(&container_name, None::<StartContainerOptions<String>>)
        .await
        .context("infra error: container lifecycle start failed: start container failed")?;

    let wait_result = wait_for_container_exit_code(docker, &container_name);
    let status = if let Some(seconds) = timeout_s {
        match tokio::time::timeout(Duration::from_secs(seconds), wait_result).await {
            Ok(result) => result?,
            Err(_) => {
                let _ = cleanup_container(docker, &container_name).await;
                return Ok(StepRunResult {
                    success: false,
                    exit_code: None,
                });
            }
        }
    } else {
        wait_result.await?
    };

    let _ = cleanup_container(docker, &container_name).await;
    Ok(StepRunResult {
        success: status == 0,
        exit_code: Some(status),
    })
}

async fn wait_for_container_exit_code(docker: &Docker, container_name: &str) -> Result<i32> {
    let mut stream = docker.wait_container(container_name, None::<WaitContainerOptions<String>>);
    let Some(result) = stream.next().await else {
        bail!("infra error: container lifecycle runtime failed: wait stream ended unexpectedly");
    };
    let result = result
        .context("infra error: container lifecycle runtime failed: waiting for container failed")?;
    let code = i32::try_from(result.status_code).unwrap_or(1);
    Ok(code)
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
