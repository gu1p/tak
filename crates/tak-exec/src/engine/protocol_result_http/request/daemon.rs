use std::{path::Path, time::Duration};

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
#[path = "daemon/errors.rs"]
pub(super) mod errors;
#[path = "daemon/resource_limits.rs"]
mod resource_limits;
#[path = "daemon/stream_upload.rs"]
mod stream_upload;

use errors::{DaemonLocalError, daemon_error, daemon_timeout};
use resource_limits::runtime_resource_limits;
pub(crate) use stream_upload::DaemonWorkspaceUploadStreamRequest;
pub(crate) use stream_upload::{StreamUploadProgress, stream_workspace_upload_via_daemon};

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
        daemon_response_to_http(response)
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
            selection: selection_value(target.remote_selection).to_string(),
            preferred_node_id: preferred_node_id(extra_headers),
            task_run_id: submit.task_run_id,
            attempt: submit.attempt,
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

fn preferred_node_id(extra_headers: &[(&str, String)]) -> Option<String> {
    extra_headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("x-tak-preferred-node"))
        .map(|(_, value)| value.clone())
        .filter(|value| !value.trim().is_empty())
}

fn selection_value(selection: tak_core::model::RemoteSelectionSpec) -> &'static str {
    match selection {
        tak_core::model::RemoteSelectionSpec::Sequential => "sequential",
        tak_core::model::RemoteSelectionSpec::RoundRobin => "round_robin",
        tak_core::model::RemoteSelectionSpec::Shuffle => "shuffle",
    }
}

async fn send_daemon_request(socket_path: &Path, request: DaemonRequest) -> Result<DaemonResponse> {
    let stream = UnixStream::connect(socket_path).await.map_err(|err| {
        anyhow::Error::new(DaemonLocalError::connect(format!(
            "Tor remote execution requires local takd serve; local takd daemon unavailable at {}: {err}",
            socket_path.display(),
        )))
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

fn daemon_response_to_http(response: DaemonResponse) -> Result<RemoteHttpResponse> {
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
        DaemonResponse::Error {
            message,
            code,
            retryable,
        } => Err(DaemonLocalError::response(message, code, retryable).into()),
        DaemonResponse::PeersSnapshot { .. } => {
            bail!("local takd daemon returned peer list for remote HTTP request")
        }
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
