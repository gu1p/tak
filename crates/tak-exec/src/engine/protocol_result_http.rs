use tak_proto::GetTaskResultResponse;

/// Fetches terminal result metadata for one remote attempt.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    _attempt: u32,
) -> Result<RemoteProtocolResult> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    let (status, response_body) =
        remote_protocol_http_request(target, "GET", &path, None, "result", Duration::from_secs(1))
            .await?;
    if status != 200 {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP {}",
            target.node_id,
            status
        );
    }

    let parsed = GetTaskResultResponse::decode(response_body.as_slice()).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for result",
            target.node_id
        )
    })?;
    let synced_outputs = parse_remote_result_outputs(target, &parsed)?;
    Ok(RemoteProtocolResult {
        success: parsed.success,
        exit_code: parsed.exit_code,
        synced_outputs,
        runtime_kind: parsed.runtime,
        runtime_engine: parsed.runtime_engine,
    })
}

/// Sends a small HTTP request to a remote endpoint and returns `(status_code, body)`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_http_request(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    phase: &str,
    timeout: Duration,
) -> Result<(u16, Vec<u8>)> {
    let socket_addr = TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;
    let header_block = remote_protocol_request_headers(&target.node_id, &target.bearer_token)?;
    let payload = body.unwrap_or(&[]);
    let request_head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {socket_addr}\r\nConnection: close\r\n{header_block}Content-Type: application/x-protobuf\r\nContent-Length: {}\r\n\r\n",
        payload.len()
    );

    let exchange = async {
        let mut stream = TransportFactory::connect(target).await?;
        stream.write_all(request_head.as_bytes()).await?;
        if !payload.is_empty() {
            stream.write_all(payload).await?;
        }
        stream.flush().await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        Ok::<Vec<u8>, anyhow::Error>(response)
    };

    let effective_timeout = TransportFactory::phase_timeout(target, timeout);
    let response_bytes = tokio::time::timeout(effective_timeout, exchange)
        .await
        .map_err(|_| {
            anyhow!(
                "infra error: remote node {} {} request timed out",
                target.node_id,
                phase
            )
        })??;
    let split = response_bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} returned malformed HTTP response for {}",
                target.node_id,
                phase
            )
        })?;
    let head = String::from_utf8_lossy(&response_bytes[..split]);
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} returned invalid HTTP status for {}",
                target.node_id,
                phase
            )
        })?;

    Ok((status_code, response_bytes[split..].to_vec()))
}
