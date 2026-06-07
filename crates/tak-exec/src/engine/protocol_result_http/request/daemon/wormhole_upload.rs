use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures::future;
use futures::io::AllowStdIo;
use magic_wormhole::transfer::send_file;
use magic_wormhole::transit::{Abilities, DEFAULT_RELAY_SERVER, RelayHint};
use magic_wormhole::{MailboxConnection, Wormhole};
use prost::Message;
use tak_proto::StartWorkspaceWormholeUploadRequest;

use super::errors::DaemonLocalError;
use super::lifecycle::request_id;
use super::stream_upload::select_upload_peer;
use super::types::{DaemonPeerSnapshot, DaemonRequest, DaemonResponse, RemoteHeader};
use super::{RemoteHttpResponse, ResponseHeader, send_daemon_request};
use crate::engine::RemoteHttpExchangeError;
use crate::engine::{StrictRemoteTarget, transport};

pub(crate) struct DaemonWorkspaceWormholeUploadRequest<'a> {
    pub(crate) target: &'a StrictRemoteTarget,
    pub(crate) upload_id: &'a str,
    pub(crate) archive_path: &'a Path,
    pub(crate) size_bytes: u64,
    pub(crate) sha256: &'a str,
    pub(crate) timeout: Duration,
}

pub(crate) struct DaemonWormholeUploadResponse {
    pub(crate) response: RemoteHttpResponse,
    pub(crate) peer_node_id: String,
}

pub(crate) async fn send_workspace_wormhole_via_daemon(
    request: DaemonWorkspaceWormholeUploadRequest<'_>,
) -> std::result::Result<DaemonWormholeUploadResponse, RemoteHttpExchangeError> {
    let target = request.target;
    let timeout = request.timeout;
    let exchange = async move {
        let peer = select_upload_peer(target).await?;
        preflight_wormhole_route(target, &peer, request.upload_id).await?;
        let response = send_wormhole_file(target, &peer, &request).await?;
        Ok(DaemonWormholeUploadResponse {
            response,
            peer_node_id: peer.node_id,
        })
    };
    tokio::time::timeout(transport::phase_timeout(target, timeout), exchange)
        .await
        .map_err(|_| super::errors::daemon_timeout(target, "workspace wormhole upload"))?
        .map_err(|err| super::errors::daemon_error(target, err))
}

async fn preflight_wormhole_route(
    target: &StrictRemoteTarget,
    peer: &DaemonPeerSnapshot,
    upload_id: &str,
) -> Result<()> {
    let response =
        forward_wormhole_request(target, peer, "GET", &wormhole_path(upload_id), Vec::new())
            .await?;
    if response.status != 200 {
        bail!(
            "workspace wormhole upload route unavailable on remote node {} with HTTP {}",
            peer.node_id,
            response.status
        );
    }
    if !marks_wormhole_support(&response) {
        bail!(
            "workspace wormhole upload route unavailable on remote node {}: missing support marker",
            peer.node_id
        );
    }
    Ok(())
}

fn marks_wormhole_support(response: &RemoteHttpResponse) -> bool {
    response.header("x-tak-workspace-transfer") == Some("wormhole")
}

async fn send_wormhole_file(
    target: &StrictRemoteTarget,
    peer: &DaemonPeerSnapshot,
    request: &DaemonWorkspaceWormholeUploadRequest<'_>,
) -> Result<RemoteHttpResponse> {
    let mailbox = MailboxConnection::create(magic_wormhole::transfer::APP_CONFIG, 2)
        .await
        .context("create workspace wormhole mailbox")?;
    let code = mailbox.code().to_string();
    let body = StartWorkspaceWormholeUploadRequest {
        upload_id: request.upload_id.to_string(),
        code,
        sha256: request.sha256.to_string(),
        size_bytes: request.size_bytes,
    }
    .encode_to_vec();
    let path = wormhole_path(request.upload_id);
    let remote_receive = forward_wormhole_request(target, peer, "POST", &path, body);
    let local_send = send_file_to_wormhole(mailbox, request.archive_path, request.size_bytes);
    let (send_result, receive_result) = tokio::join!(local_send, remote_receive);
    send_result?;
    receive_result
}

async fn send_file_to_wormhole(
    mailbox: MailboxConnection<magic_wormhole::transfer::AppVersion>,
    archive_path: &Path,
    size_bytes: u64,
) -> Result<()> {
    let wormhole = Wormhole::connect(mailbox)
        .await
        .context("connect workspace wormhole")?;
    let file = std::fs::File::open(archive_path)
        .with_context(|| format!("open workspace archive {}", archive_path.display()))?;
    let mut file = AllowStdIo::new(file);
    send_file(
        wormhole,
        relay_hints()?,
        &mut file,
        "workspace.zip",
        size_bytes,
        transfer_abilities("TAK_WORMHOLE_TRANSIT"),
        |info| tracing::info!(transit = %info, "workspace wormhole transit established"),
        |_sent, _total| {},
        future::pending::<()>(),
    )
    .await
    .context("send workspace archive over wormhole")
}

async fn forward_wormhole_request(
    target: &StrictRemoteTarget,
    peer: &DaemonPeerSnapshot,
    method: &str,
    path: &str,
    body: Vec<u8>,
) -> Result<RemoteHttpResponse> {
    let request = DaemonRequest::ForwardRemoteHttp {
        request_id: request_id("wormhole-upload", target, path),
        node_id: peer.node_id.clone(),
        method: method.to_string(),
        path: path.to_string(),
        headers: Vec::new(),
        body,
    };
    match send_daemon_request(&transport::broker_socket_path(), request).await? {
        DaemonResponse::RemoteHttpResponse {
            status,
            headers,
            body,
        } => Ok(RemoteHttpResponse {
            status,
            headers: response_headers(headers),
            body,
            daemon_task_handle: None,
            daemon_peer_node_id: Some(peer.node_id.clone()),
            daemon_peer_endpoint: Some(peer.endpoint.clone()),
        }),
        DaemonResponse::Error {
            message,
            code,
            retryable,
        } => Err(DaemonLocalError::response(message, code, retryable).into()),
        _ => bail!("local takd returned unexpected response for workspace wormhole upload"),
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

fn relay_hints() -> Result<Vec<RelayHint>> {
    let relay = DEFAULT_RELAY_SERVER
        .parse()
        .context("parse default magic-wormhole relay URL")?;
    Ok(vec![
        RelayHint::from_urls(None, [relay]).context("create default magic-wormhole relay hint")?,
    ])
}

fn transfer_abilities(env_name: &str) -> Abilities {
    match std::env::var(env_name).unwrap_or_default().trim() {
        "relay" => Abilities::FORCE_RELAY,
        _ => Abilities::ALL,
    }
}

fn wormhole_path(upload_id: &str) -> String {
    format!("/v2/workspaces/uploads/{upload_id}/wormhole")
}
