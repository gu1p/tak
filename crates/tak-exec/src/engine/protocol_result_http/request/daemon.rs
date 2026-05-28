use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message;
use tak_proto::SubmitTaskRequest;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::super::super::{RemoteHttpExchangeError, StrictRemoteTarget, transport};
use super::{RemoteHttpResponse, ResponseHeader};

#[path = "daemon/types.rs"]
mod types;

use types::{DaemonRequest, DaemonResponse, PeerEligibility, RemoteHeader};
#[path = "daemon/lifecycle.rs"]
mod lifecycle;

use lifecycle::{lifecycle_request, request_id};

pub(super) async fn request_via_daemon(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: Option<&[u8]>,
    phase: &str,
    timeout: Duration,
    extra_headers: &[(&str, String)],
) -> std::result::Result<RemoteHttpResponse, RemoteHttpExchangeError> {
    let request = daemon_request_for(target, method, path, body.unwrap_or(&[]), extra_headers)
        .map_err(|err| RemoteHttpExchangeError::other(format!("{err:#}")))?;
    let exchange = async {
        let response = send_daemon_request(&transport::broker_socket_path(), request).await?;
        daemon_response_to_http(target, response)
    };
    tokio::time::timeout(timeout, exchange)
        .await
        .map_err(|_| daemon_timeout(target, phase))?
        .map_err(|err| daemon_error(target, err))
}

fn daemon_request_for(
    target: &StrictRemoteTarget,
    method: &str,
    path: &str,
    body: &[u8],
    extra_headers: &[(&str, String)],
) -> Result<DaemonRequest> {
    if method == "POST" && path == "/v1/tasks/submit" {
        let submit = SubmitTaskRequest::decode(body)
            .context("failed to decode submit payload for daemon placement")?;
        return Ok(DaemonRequest::PlaceRemote {
            request_id: request_id("place", target, path),
            requirements: node_requirements(target),
            task_run_id: submit.task_run_id,
            submit_body: body.to_vec(),
        });
    }
    if let Some(request) = lifecycle_request(target, method, path, extra_headers)? {
        return Ok(request);
    }
    Ok(DaemonRequest::ForwardRemoteHttp {
        request_id: request_id("forward", target, path),
        node_id: target.node_id.clone(),
        method: method.to_string(),
        path: path.to_string(),
        headers: request_headers(extra_headers),
        body: body.to_vec(),
    })
}

async fn send_daemon_request(socket_path: &Path, request: DaemonRequest) -> Result<DaemonResponse> {
    let stream = UnixStream::connect(socket_path).await.with_context(|| {
        format!(
            "Tor remote execution requires local takd serve; local takd daemon unavailable at {}",
            socket_path.display()
        )
    })?;
    let payload = serde_json::to_string(&request)?;
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half.write_all(payload.as_bytes()).await?;
    writer_half.write_all(b"\n").await?;
    writer_half.flush().await?;

    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        bail!("daemon closed connection before response");
    }
    serde_json::from_str(line.trim_end()).context("failed to decode daemon response")
}

fn daemon_response_to_http(
    target: &StrictRemoteTarget,
    response: DaemonResponse,
) -> Result<RemoteHttpResponse> {
    match response {
        DaemonResponse::RemotePlaced {
            task_handle,
            peer,
            status,
            headers,
            body,
        } => Ok(RemoteHttpResponse {
            status,
            headers: response_headers(headers),
            body,
            daemon_task_handle: Some(task_handle),
            daemon_peer_node_id: Some(peer.node_id),
            daemon_peer_endpoint: Some(peer.endpoint),
        }),
        DaemonResponse::RemoteHttpResponse {
            status,
            headers,
            body,
        } => Ok(RemoteHttpResponse {
            status,
            headers: response_headers(headers),
            body,
            daemon_task_handle: None,
            daemon_peer_node_id: None,
            daemon_peer_endpoint: None,
        }),
        DaemonResponse::Error { message } => bail!(
            "local takd daemon failed while contacting remote node {}: {message}",
            target.node_id
        ),
    }
}

fn node_requirements(target: &StrictRemoteTarget) -> PeerEligibility {
    let (cpu_cores, memory_mb) = runtime_resource_limits(target);
    PeerEligibility {
        pool: target.required_pool.clone(),
        tags: target.required_tags.clone(),
        capabilities: target.required_capabilities.clone(),
        transport: Some("tor".to_string()),
        cpu_cores,
        memory_mb,
    }
}

fn runtime_resource_limits(target: &StrictRemoteTarget) -> (Option<f64>, Option<u64>) {
    let Some(tak_core::model::RemoteRuntimeSpec::Containerized {
        resource_limits: Some(limits),
        ..
    }) = target.runtime.as_ref()
    else {
        return (None, None);
    };
    (limits.cpu_cores, limits.memory_mb)
}

fn response_headers(headers: Vec<RemoteHeader>) -> Vec<ResponseHeader> {
    headers
        .into_iter()
        .map(|header| ResponseHeader {
            name: header.name,
            value: header.value,
        })
        .collect()
}

fn request_headers(headers: &[(&str, String)]) -> Vec<RemoteHeader> {
    headers
        .iter()
        .map(|(name, value)| RemoteHeader {
            name: (*name).to_string(),
            value: value.clone(),
        })
        .collect()
}

fn daemon_timeout(target: &StrictRemoteTarget, phase: &str) -> RemoteHttpExchangeError {
    RemoteHttpExchangeError::timeout(format!(
        "infra error: local takd daemon timed out while contacting remote node {} for {}",
        target.node_id, phase
    ))
}

fn daemon_error(target: &StrictRemoteTarget, err: anyhow::Error) -> RemoteHttpExchangeError {
    let message = format!("{err:#}");
    if message.contains("connect_failed") || message.contains("unavailable") {
        return RemoteHttpExchangeError::connect(format!(
            "infra error: remote node {} unavailable via local takd daemon at {}: {message}",
            target.node_id,
            transport::broker_socket_path().display()
        ));
    }
    RemoteHttpExchangeError::other(format!(
        "infra error: local takd daemon rejected request at {} while contacting remote node {}: {message}",
        transport::broker_socket_path().display(),
        target.node_id
    ))
}
