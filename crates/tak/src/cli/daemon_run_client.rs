async fn try_run_via_daemon(
    socket_path: PathBuf,
    workspace_root: PathBuf,
    targets: &[TaskLabel],
    jobs: usize,
    keep_going: bool,
) -> Result<bool> {
    let stream = match UnixStream::connect(&socket_path).await {
        Ok(stream) => stream,
        Err(_) => return Ok(false),
    };

    let request = Request::RunTasks(RunTasksRequest {
        request_id: Uuid::new_v4().to_string(),
        workspace_root: workspace_root.display().to_string(),
        labels: targets.iter().map(ToString::to_string).collect(),
        jobs,
        keep_going,
        lease_socket: Some(socket_path.display().to_string()),
        lease_ttl_ms: env_u64("TAK_LEASE_TTL_MS", 30_000),
        lease_poll_interval_ms: env_u64("TAK_LEASE_POLL_MS", 200),
        session_id: std::env::var("TAK_SESSION_ID").ok(),
        user: std::env::var("TAK_USER").ok(),
    });

    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            bail!("daemon closed connection before run completed");
        }

        match serde_json::from_str::<Response>(line.trim_end())? {
            Response::RunStarted { .. } => {}
            Response::RunTaskResult {
                label,
                attempts,
                success,
                exit_code,
                placement,
                remote_node,
                transport,
                reason,
                context_hash,
                runtime,
                runtime_engine,
                ..
            } => {
                println!(
                    "{label}: {} (attempts={}, exit_code={}, placement={}, remote_node={}, transport={}, reason={}, context_hash={}, runtime={}, runtime_engine={})",
                    if success { "ok" } else { "failed" },
                    attempts,
                    exit_code.map_or_else(|| "none".to_string(), |code| code.to_string()),
                    placement,
                    remote_node.as_deref().unwrap_or("none"),
                    transport.as_deref().unwrap_or("none"),
                    reason.as_deref().unwrap_or("none"),
                    context_hash.as_deref().unwrap_or("none"),
                    runtime.as_deref().unwrap_or("none"),
                    runtime_engine.as_deref().unwrap_or("none")
                );
            }
            Response::RunCompleted { .. } => return Ok(true),
            Response::Error { message, .. } => bail!("daemon error: {message}"),
            other => bail!("unexpected daemon response: {other:?}"),
        }
    }
}
