use http_body_util::BodyExt;
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

struct AbortOnDrop<T> {
    handle: Option<tokio::task::JoinHandle<T>>,
}

impl<T> AbortOnDrop<T> {
    fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
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
) -> std::result::Result<(u16, Vec<u8>), RemoteHttpExchangeError> {
    let socket_addr = TransportFactory::socket_addr(target)
        .with_context(|| {
            format!(
                "infra error: remote node {} has invalid endpoint {}",
                target.node_id, target.endpoint
            )
        })
        .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?;
    let bearer_token = remote_protocol_bearer_token(
        &target.node_id,
        &target.bearer_token,
        target.transport_kind,
    )
    .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?;
    let payload = body.unwrap_or(&[]);

    let exchange = async {
        let stream = TransportFactory::connect(target)
            .await
            .map_err(|err| RemoteHttpExchangeError::connect(format!("{err:#}")))?;
        let (mut sender, connection) =
            hyper::client::conn::http1::handshake(hyper_util::rt::TokioIo::new(stream))
                .await
                .map_err(|_| {
                    RemoteHttpExchangeError::other(format!(
                        "infra error: remote node {} returned malformed HTTP response for {}",
                        target.node_id, phase
                    ))
                })?;
        let _connection_task = AbortOnDrop::new(tokio::spawn(async move {
            let _ = connection.await;
        }));
        let mut request = hyper::Request::builder()
            .method(method)
            .uri(path)
            .header(hyper::header::HOST, socket_addr.as_str())
            .header(hyper::header::CONNECTION, "close")
            .header("X-Tak-Protocol-Version", "v1")
            .header(hyper::header::CONTENT_TYPE, "application/x-protobuf");
        if let Some(bearer_token) = bearer_token {
            request = request.header(
                hyper::header::AUTHORIZATION,
                format!("Bearer {bearer_token}"),
            );
        }
        let request = request
            .body(http_body_util::Full::new(bytes::Bytes::copy_from_slice(payload)))
            .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?;
        let response = sender.send_request(request).await.map_err(|_| {
            RemoteHttpExchangeError::other(format!(
                "infra error: remote node {} returned malformed HTTP response for {}",
                target.node_id, phase
            ))
        })?;
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|_| {
                RemoteHttpExchangeError::other(format!(
                    "infra error: remote node {} returned truncated HTTP body for {}",
                    target.node_id, phase
                ))
            })?
            .to_bytes()
            .to_vec();
        Ok((status, body))
    };

    let effective_timeout = TransportFactory::phase_timeout(target, timeout);
    tokio::time::timeout(effective_timeout, exchange)
        .await
        .map_err(|_| {
            RemoteHttpExchangeError::timeout(format!(
                "infra error: remote node {} at {} via {} {} request timed out",
                target.node_id,
                target.endpoint,
                target.transport_kind.as_result_value(),
                phase
            ))
        })?
}
