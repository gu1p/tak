type ContainerLogTask = Option<tokio::task::JoinHandle<Result<()>>>;

fn spawn_container_log_task(
    docker: Docker,
    container_id: String,
    task_label: TaskLabel,
    attempt: u32,
    output_observer: Option<Arc<dyn TaskOutputObserver>>,
) -> ContainerLogTask {
    let output_observer = output_observer?;

    Some(tokio::spawn(async move {
        let mut logs = docker.logs::<String>(
            &container_id,
            Some(LogsOptions {
                follow: true,
                stdout: true,
                stderr: true,
                timestamps: false,
                tail: "all".to_string(),
                ..Default::default()
            }),
        );
        while let Some(item) = logs.next().await {
            let item = item.context(
                "infra error: container lifecycle runtime failed: log streaming failed",
            )?;
            if let Some((stream, bytes)) = container_log_output_chunk(item) {
                crate::emit_task_output(
                    Some(&output_observer),
                    &task_label,
                    attempt,
                    stream,
                    &bytes,
                )?;
            }
        }
        Ok(())
    }))
}

fn container_log_output_chunk(output: LogOutput) -> Option<(OutputStream, Vec<u8>)> {
    match output {
        LogOutput::StdOut { message } => Some((OutputStream::Stdout, message.to_vec())),
        LogOutput::StdErr { message } => Some((OutputStream::Stderr, message.to_vec())),
        LogOutput::Console { message } => Some((OutputStream::Stdout, message.to_vec())),
        LogOutput::StdIn { message } => Some((OutputStream::Stdout, message.to_vec())),
    }
}

async fn finish_container_log_task(task: ContainerLogTask) -> Result<()> {
    let Some(task) = task else {
        return Ok(());
    };
    task.await.context("container log task failed to join")?
}
