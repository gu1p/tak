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
    let _ = attempt;
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

    let parsed: serde_json::Value = serde_json::from_str(&response_body).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid JSON for result",
            target.node_id
        )
    })?;
    let success = parsed
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} result missing boolean success field",
                target.node_id
            )
        })?;
    let exit_code = parsed
        .get("exit_code")
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok());

    if let Some(sync_mode) = parsed.get("sync_mode").and_then(serde_json::Value::as_str)
        && sync_mode != "OUTPUTS_AND_LOGS"
    {
        bail!(
            "infra error: remote node {} result sync mode `{sync_mode}` is unsupported in V1; expected `OUTPUTS_AND_LOGS`",
            target.node_id
        );
    }

    let synced_outputs = parse_remote_result_outputs(target, &parsed)?;
    Ok(RemoteProtocolResult {
        success,
        exit_code,
        synced_outputs,
        runtime_kind: parsed
            .get("runtime")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        runtime_engine: parsed
            .get("runtime_engine")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
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
    body: Option<&str>,
    phase: &str,
    timeout: Duration,
) -> Result<(u16, String)> {
    let socket_addr = TransportFactory::socket_addr(target).with_context(|| {
        format!(
            "infra error: remote node {} has invalid endpoint {}",
            target.node_id, target.endpoint
        )
    })?;
    let header_block =
        remote_protocol_request_headers(&target.node_id, target.service_auth_env.as_deref())?;
    let payload = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {socket_addr}\r\nConnection: close\r\n{header_block}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    );

    let exchange = async {
        let mut stream = TransportFactory::connect(target).await?;
        stream.write_all(request.as_bytes()).await?;
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

    let response_text = String::from_utf8_lossy(&response_bytes);
    let (head, body) = response_text.split_once("\r\n\r\n").ok_or_else(|| {
        anyhow!(
            "infra error: remote node {} returned malformed HTTP response for {}",
            target.node_id,
            phase
        )
    })?;
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

    Ok((status_code, body.to_string()))
}
