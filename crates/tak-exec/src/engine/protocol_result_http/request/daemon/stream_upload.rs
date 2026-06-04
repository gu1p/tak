use std::path::Path;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::net::UnixStream;

#[path = "stream_upload/progress.rs"]
mod progress;
#[path = "stream_upload/response.rs"]
mod response;
#[path = "stream_upload/retry.rs"]
mod retry;
#[path = "stream_upload/status.rs"]
mod status;

use super::errors;
use super::lifecycle::request_id;
use super::types::{DaemonPeerSnapshot, DaemonRequest, DaemonResponse};
use super::{RemoteHttpResponse, node_requirements, send_daemon_request};
use crate::engine::RemoteHttpExchangeError;
use crate::engine::{StrictRemoteTarget, transport};

use progress::ActiveStreamUploadProgress;
pub(crate) use progress::StreamUploadProgress;
use response::read_raw_http_response;
use retry::StreamUploadPlan;

pub(crate) struct DaemonStreamUploadResponse {
    pub(crate) response: RemoteHttpResponse,
    pub(crate) peer_node_id: String,
}

pub(crate) struct DaemonWorkspaceUploadStreamRequest<'a> {
    pub(crate) target: &'a StrictRemoteTarget,
    pub(crate) upload_id: &'a str,
    pub(crate) archive_path: &'a Path,
    pub(crate) offset: u64,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: &'a str,
    pub(crate) timeout: Duration,
    pub(crate) progress: Option<StreamUploadProgress<'a>>,
}

pub(crate) async fn stream_workspace_upload_via_daemon(
    request: DaemonWorkspaceUploadStreamRequest<'_>,
) -> std::result::Result<DaemonStreamUploadResponse, RemoteHttpExchangeError> {
    let target = request.target;
    let timeout = request.timeout;
    let plan = StreamUploadPlan::from_request(&request);
    let progress_input = request.progress;
    let exchange = async move {
        let peer = select_upload_peer(target).await?;
        let mut progress =
            progress_input.map(|input| ActiveStreamUploadProgress::new(input, plan.size_bytes()));
        let response = retry::stream_until_complete(&plan, &peer, progress.as_mut()).await?;
        Ok(DaemonStreamUploadResponse {
            response,
            peer_node_id: peer.node_id,
        })
    };
    tokio::time::timeout(transport::phase_timeout(target, timeout), exchange)
        .await
        .map_err(|_| errors::daemon_timeout(target, "workspace upload stream"))?
        .map_err(|err| errors::daemon_error(target, err))
}

async fn select_upload_peer(target: &StrictRemoteTarget) -> Result<DaemonPeerSnapshot> {
    let request = DaemonRequest::PeersEligible {
        request_id: request_id("upload-peers", target, "/v2/workspaces/uploads"),
        requirements: node_requirements(target),
    };
    match send_daemon_request(&transport::broker_socket_path(), request).await? {
        DaemonResponse::PeersSnapshot { peers } => peers
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("all Tor peers are unreachable for workspace upload")),
        DaemonResponse::Error { message } => bail!("{message}"),
        _ => bail!("local takd returned unexpected response for peer selection"),
    }
}

async fn send_stream_upload_request(
    peer: &DaemonPeerSnapshot,
    path: &str,
    archive_path: &Path,
    offset: u64,
    size_bytes: u64,
    sha256: &str,
    progress: Option<&mut ActiveStreamUploadProgress<'_>>,
) -> Result<RemoteHttpResponse> {
    let mut stream = UnixStream::connect(transport::broker_socket_path()).await?;
    let remaining = size_bytes.saturating_sub(offset);
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {}\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: {}\r\nX-Tak-Remote-Endpoint: {}\r\nX-Tak-Remote-Protocol: h2\r\nX-Tak-Remote-Transport: tor\r\nX-Tak-Upload-Sha256: {sha256}\r\nX-Tak-Upload-Size: {size_bytes}\r\nContent-Length: {remaining}\r\nConnection: close\r\n\r\n",
        peer.endpoint, peer.node_id, peer.endpoint
    );
    stream.write_all(head.as_bytes()).await?;
    copy_archive_body(
        peer,
        archive_path,
        offset,
        size_bytes,
        &mut stream,
        progress,
    )
    .await?;
    read_raw_http_response(peer, stream).await
}

async fn copy_archive_body(
    peer: &DaemonPeerSnapshot,
    archive_path: &Path,
    offset: u64,
    size_bytes: u64,
    stream: &mut UnixStream,
    progress: Option<&mut ActiveStreamUploadProgress<'_>>,
) -> Result<()> {
    let mut progress = progress;
    let remaining = size_bytes.saturating_sub(offset);
    let mut file = tokio::fs::File::open(archive_path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut sent = 0_u64;
    if let Some(progress) = progress.as_mut() {
        progress.report(&peer.node_id, offset, true)?;
    }
    while sent < remaining {
        let limit = buffer.len().min((remaining - sent) as usize);
        let read = file.read(&mut buffer[..limit]).await?;
        if read == 0 {
            bail!("staged workspace archive ended before declared upload size");
        }
        stream.write_all(&buffer[..read]).await?;
        sent += read as u64;
        if let Some(progress) = progress.as_mut() {
            progress.report(&peer.node_id, offset + sent, false)?;
        }
    }
    stream.flush().await?;
    if let Some(progress) = progress.as_mut() {
        progress.report(&peer.node_id, size_bytes, true)?;
    }
    Ok(())
}
