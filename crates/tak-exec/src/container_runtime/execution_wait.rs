use super::*;

pub(super) async fn wait_for_container_step(
    docker: &Docker,
    engine: ContainerEngine,
    podman_wait_socket: Option<&str>,
    container_id: &str,
    timeout_s: Option<u64>,
    cancellation: &RunCancellation,
) -> Result<Option<i32>> {
    let wait = wait_for_container_exit_code(docker, engine, podman_wait_socket, container_id);
    tokio::pin!(wait);
    // Cancellation is authoritative: poll it first (`biased`) and, even if the
    // wait future also became ready in the same tick (e.g. the container's wait
    // stream resolved because cancellation tore the container down), still report
    // the run as cancelled rather than as a spurious exit/failure.
    if let Some(seconds) = timeout_s {
        let timeout = tokio::time::sleep(Duration::from_secs(seconds));
        tokio::pin!(timeout);
        return tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(crate::engine::cancelled_error()),
            result = &mut wait => wait_outcome(cancellation, result),
            _ = &mut timeout => Ok(None),
        };
    }
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(crate::engine::cancelled_error()),
        result = &mut wait => wait_outcome(cancellation, result),
    }
}

fn wait_outcome(cancellation: &RunCancellation, result: Result<i32>) -> Result<Option<i32>> {
    if cancellation.is_cancelled() {
        return Err(crate::engine::cancelled_error());
    }
    Ok(Some(result?))
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
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .parse::<i32>()
        .context("infra error: container lifecycle runtime failed: invalid podman wait exit code")
}

pub(super) async fn cleanup_container(docker: &Docker, container_name: &str) -> Result<()> {
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
