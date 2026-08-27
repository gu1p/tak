use anyhow::{Result, bail};

use super::errors::{DaemonLocalError, peer_error};
use super::types::{DaemonResponse, RemoteHeader};
use super::{RemoteHttpResponse, ResponseHeader};

pub(super) fn daemon_response_to_http(response: DaemonResponse) -> Result<RemoteHttpResponse> {
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
            node_id,
        } => {
            let error: anyhow::Error = DaemonLocalError::response(message, code, retryable).into();
            match node_id {
                Some(node_id) => Err(peer_error(&node_id, error)),
                None => Err(error),
            }
        }
        DaemonResponse::PeersSnapshot { .. } => {
            bail!("local takd daemon returned peer list for remote HTTP request")
        }
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
