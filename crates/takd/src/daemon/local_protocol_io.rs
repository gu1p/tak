async fn handle_client(stream: UnixStream, manager: SharedLeaseManager) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let request: Request = serde_json::from_str(line.trim_end())
            .with_context(|| format!("invalid request line: {}", line.trim_end()))?;
        match request {
            Request::RunTasks(payload) => {
                write_protocol_response(
                    &mut writer_half,
                    &Response::RunStarted {
                        request_id: payload.request_id.clone(),
                    },
                )
                .await?;
                match execute_run_tasks_request(&payload).await {
                    Ok(task_results) => {
                        for task_result in task_results {
                            write_protocol_response(
                                &mut writer_half,
                                &Response::RunTaskResult {
                                    request_id: payload.request_id.clone(),
                                    label: task_result.label,
                                    attempts: task_result.attempts,
                                    success: task_result.success,
                                    exit_code: task_result.exit_code,
                                    placement: task_result.placement,
                                    remote_node: task_result.remote_node,
                                    transport: task_result.transport,
                                    reason: task_result.reason,
                                    context_hash: task_result.context_hash,
                                    runtime: task_result.runtime,
                                    runtime_engine: task_result.runtime_engine,
                                },
                            )
                            .await?;
                        }
                        write_protocol_response(
                            &mut writer_half,
                            &Response::RunCompleted {
                                request_id: payload.request_id,
                            },
                        )
                        .await?;
                    }
                    Err(err) => {
                        write_protocol_response(
                            &mut writer_half,
                            &Response::Error {
                                request_id: payload.request_id,
                                message: err.to_string(),
                            },
                        )
                        .await?;
                    }
                }
            }
            other => {
                let response = dispatch_request(other, &manager)?;
                write_protocol_response(&mut writer_half, &response).await?;
            }
        }
    }

    Ok(())
}

async fn write_protocol_response(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    response: &Response,
) -> Result<()> {
    let encoded = serde_json::to_string(response)?;
    writer.write_all(encoded.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}
