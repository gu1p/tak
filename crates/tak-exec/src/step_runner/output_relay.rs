fn spawn_output_relay<R>(
    reader: Option<R>,
    task_label: TaskLabel,
    attempt: u32,
    stream: OutputStream,
    output_observer: Option<Arc<dyn TaskOutputObserver>>,
) -> OutputRelayTask
where
    R: AsyncRead + Unpin + Send + 'static,
{
    let reader = reader?;
    let output_observer = output_observer?;

    Some(tokio::spawn(async move {
        relay_child_output(reader, task_label, attempt, stream, output_observer).await
    }))
}

async fn relay_child_output<R>(
    mut reader: R,
    task_label: TaskLabel,
    attempt: u32,
    stream: OutputStream,
    output_observer: Arc<dyn TaskOutputObserver>,
) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .context("failed reading child process output")?;
        if read == 0 {
            return Ok(());
        }
        crate::emit_task_output(
            Some(&output_observer),
            &task_label,
            attempt,
            stream,
            &buffer[..read],
        )?;
    }
}

async fn finish_output_relays(
    stdout_task: OutputRelayTask,
    stderr_task: OutputRelayTask,
) -> Result<()> {
    await_output_relay(stdout_task).await?;
    await_output_relay(stderr_task).await
}

async fn await_output_relay(task: OutputRelayTask) -> Result<()> {
    let Some(task) = task else {
        return Ok(());
    };
    task.await.context("output relay task failed to join")?
}
