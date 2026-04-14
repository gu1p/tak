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
    attempt: u32,
) -> Result<RemoteProtocolResult> {
    let Some(result) = try_remote_protocol_result(target, task_run_id, attempt).await? else {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP 404",
            target.node_id
        );
    };
    Ok(result)
}

async fn try_remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    _attempt: u32,
) -> Result<Option<RemoteProtocolResult>> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    let (status, response_body) =
        remote_protocol_http_request(target, "GET", &path, None, "result", Duration::from_secs(1))
            .await?;
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP {}",
            target.node_id,
            status
        );
    }
    Ok(Some(parse_remote_protocol_result(target, &response_body)?))
}

fn parse_remote_protocol_result(
    target: &StrictRemoteTarget,
    response_body: &[u8],
) -> Result<RemoteProtocolResult> {
    let parsed = GetTaskResultResponse::decode(response_body).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for result",
            target.node_id
        )
    })?;
    let synced_outputs = parse_remote_result_outputs(target, &parsed)?;
    Ok(RemoteProtocolResult {
        success: parsed.success,
        exit_code: parsed.exit_code,
        failure_detail: (!parsed.success)
            .then_some(parsed.stderr_tail)
            .flatten()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
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
        read_http_response(&mut stream, target, phase).await
    };

    let effective_timeout = TransportFactory::phase_timeout(target, timeout);
    tokio::time::timeout(effective_timeout, exchange)
        .await
        .map_err(|_| {
            anyhow!(
                "infra error: remote node {} at {} via {} {} request timed out",
                target.node_id,
                target.endpoint,
                target.transport_kind.as_result_value(),
                phase
            )
        })?
}

async fn read_http_response<S>(
    stream: &mut S,
    target: &StrictRemoteTarget,
    phase: &str,
) -> Result<(u16, Vec<u8>)>
where
    S: tokio::io::AsyncRead + Unpin + ?Sized,
{
    let mut response = Vec::new();
    let mut chunk = [0_u8; 1024];
    let split = loop {
        if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            bail!(
                "infra error: remote node {} returned malformed HTTP response for {}",
                target.node_id,
                phase
            );
        }
        response.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&response[..split]);
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
    let content_length = response_content_length(&head, target, phase)?;
    let mut body = response[split..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            bail!(
                "infra error: remote node {} returned truncated HTTP body for {}",
                target.node_id,
                phase
            );
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Ok((status_code, body))
}

fn response_content_length(
    head: &str,
    target: &StrictRemoteTarget,
    phase: &str,
) -> Result<usize> {
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value.trim().parse::<usize>().with_context(|| {
                format!(
                    "infra error: remote node {} returned invalid content-length for {}",
                    target.node_id, phase
                )
            });
        }
    }

    Ok(0)
}
