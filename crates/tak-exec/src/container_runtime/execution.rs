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
            nano_cpus: container_nano_cpus(executor.resource_limits),
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
    let mut labels = HashMap::from([
        ("tak.owner".to_string(), identity.owner.clone()),
        ("tak.submit_key".to_string(), identity.submit_key.clone()),
        ("tak.task_run_id".to_string(), identity.task_run_id.clone()),
        ("tak.attempt".to_string(), run_context.attempt.to_string()),
        (
            "tak.task_label".to_string(),
            run_context.task_label.to_string(),
        ),
    ]);
    // A nonzero step timeout is wall-clock: the daemon must not pause such a
    // container (its timeout keeps running while frozen). Expose it as a label
    // so the memory-pressure controller can skip it.
    if let Some(timeout_s) = run_context.timeout_s
        && timeout_s > 0
    {
        labels.insert("tak.timeout_s".to_string(), timeout_s.to_string());
    }
    Some(labels)
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
        message: exit_137_diagnostic_message(oom_state, executor.resource_limits),
    })
}

/// CPU quota (`nano_cpus`) for the container, derived from the task's declared
/// CPU reservation: `cpu_cores` CPUs == `cpu_cores * 1e9` nano-CPUs. Bounds CPU
/// usage and makes Rust's cgroup-aware `available_parallelism()` report the
/// reserved core count inside the container, taming default test/codegen
/// parallelism. `None` when no CPU reservation is declared.
///
/// ```no_run
/// # // Reason: private crate-internal helper, not reachable via `use`.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn container_nano_cpus(limits: Option<&ContainerResourceLimitsSpec>) -> Option<i64> {
    let cpu_cores = limits?.cpu_cores?;
    if !cpu_cores.is_finite() || cpu_cores <= 0.0 {
        return None;
    }
    Some((cpu_cores * 1_000_000_000.0).round() as i64)
}

async fn container_oom_killed(docker: &Docker, container_id: &str) -> Option<bool> {
    docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()
        .and_then(|container| container.state)
        .and_then(|state| state.oom_killed)
}

fn exit_137_diagnostic_message(
    oom_killed: Option<bool>,
    limits: Option<&ContainerResourceLimitsSpec>,
) -> String {
    let oom_state = match oom_killed {
        Some(value) => format!("OOMKilled={value}"),
        None => "OOMKilled=unknown".to_string(),
    };
    let throttling = match limits.and_then(|limits| limits.cpu_cores) {
        Some(cpu_cores) => {
            let thread_cap = (cpu_cores.floor() as u64).max(1);
            format!(
                "container CPU is throttled to {cpu_cores} core(s) and test/codegen parallelism is capped to {thread_cap} thread(s)"
            )
        }
        None => "no container CPU/parallelism throttling is applied".to_string(),
    };
    format!(
        "container exited with exit code 137 ({oom_state}); {throttling}. \
         Tak does not hard-cap container memory and never kills a container for over-using \
         memory, so a 137 here is a host-level SIGKILL (kernel OOM or systemd-oomd) under memory \
         pressure — reduce node concurrency or add host swap; inspect `dmesg -T | grep -i oom` \
         and `journalctl -u systemd-oomd` on the worker"
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
