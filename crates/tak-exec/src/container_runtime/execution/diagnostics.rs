use super::*;

pub(super) async fn emit_exit_137_diagnostic(
    executor: &ContainerStepExecutor<'_>,
    run_context: &ContainerStepRunContext<'_>,
    container_id: &str,
) -> Result<Option<bool>> {
    let oom_state = container_oom_killed(executor.docker, container_id).await;
    let Some(observer) = run_context.output_observer else {
        return Ok(oom_state);
    };
    observer.observe_status(TaskStatusEvent {
        task_label: run_context.task_label.clone(),
        attempt: run_context.attempt,
        phase: TaskStatusPhase::RemoteWait,
        remote_node_id: None,
        message: exit_137_diagnostic_message(oom_state, executor.resource_limits),
    })?;
    Ok(oom_state)
}

async fn container_oom_killed(docker: &Docker, container_id: &str) -> Option<bool> {
    docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|container| container.state)
        .and_then(|state| state.oom_killed)
}

pub(in crate::container_runtime) fn exit_137_diagnostic_message(
    oom_killed: Option<bool>,
    _reservation: Option<&ContainerResourceLimitsSpec>,
) -> String {
    let oom_state = match oom_killed {
        Some(value) => format!("OOMKilled={value}"),
        None => "OOMKilled=unknown".to_string(),
    };
    match oom_killed {
        Some(true) => format!(
            "container exited with exit code 137 ({oom_state}); container OOM confirmed by container-engine evidence"
        ),
        Some(false) => format!(
            "container exited with exit code 137 ({oom_state}); the cause is unknown because the container engine did not attribute the termination to a container OOM"
        ),
        None => format!(
            "container exited with exit code 137 ({oom_state}); the cause is unknown because container-engine evidence was unavailable"
        ),
    }
}

pub(super) fn finish_container_step(
    step_result: Result<StepRunResult>,
    cleanup_result: Result<()>,
    log_result: Result<()>,
) -> Result<StepRunResult> {
    match step_result {
        Err(error) => match cleanup_result {
            Err(cleanup_error) => Err(error.context(format!("{cleanup_error:#}"))),
            Ok(()) => Err(error),
        },
        Ok(result) => {
            if let Err(error) = cleanup_result {
                tracing::warn!("container cleanup was not confirmed: {error:#}");
            }
            log_result?;
            Ok(result)
        }
    }
}
