use anyhow::{Context, Result, bail};
use prost::Message;
use tak_proto::{AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse};

use super::super::lifecycle::request_id;
use super::super::send_daemon_request;
use super::super::types::{DaemonPeerSnapshot, DaemonRequest, DaemonResponse};
use crate::engine::protocol_result_http::request::RemoteHttpResponse;
use crate::engine::{StrictRemoteTarget, transport};

pub(super) async fn upload_status(
    target: &StrictRemoteTarget,
    peer: &DaemonPeerSnapshot,
    upload_id: &str,
) -> Result<BeginWorkspaceUploadResponse> {
    let path = status_path(upload_id);
    let request = DaemonRequest::ForwardRemoteHttp {
        request_id: request_id("upload-status", target, &path),
        node_id: peer.node_id.clone(),
        method: "GET".to_string(),
        path,
        headers: Vec::new(),
        body: Vec::new(),
    };
    match send_daemon_request(&transport::broker_socket_path(), request).await? {
        DaemonResponse::RemoteHttpResponse {
            status: 200, body, ..
        } => BeginWorkspaceUploadResponse::decode(body.as_slice())
            .context("decode workspace upload status response"),
        DaemonResponse::RemoteHttpResponse { status, .. } => {
            bail!("workspace upload status failed with HTTP {status}")
        }
        DaemonResponse::Error { message } => bail!("{message}"),
        _ => bail!("local takd returned unexpected response for upload status"),
    }
}

pub(super) fn completed_status_response(
    peer: &DaemonPeerSnapshot,
    status: BeginWorkspaceUploadResponse,
) -> RemoteHttpResponse {
    RemoteHttpResponse {
        status: 200,
        headers: Vec::new(),
        body: AppendWorkspaceUploadResponse {
            offset: status.offset,
            complete: true,
        }
        .encode_to_vec(),
        daemon_task_handle: None,
        daemon_peer_node_id: Some(peer.node_id.clone()),
        daemon_peer_endpoint: Some(peer.endpoint.clone()),
    }
}

fn status_path(upload_id: &str) -> String {
    format!("/v2/workspaces/uploads/{upload_id}")
}
