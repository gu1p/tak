use super::*;

pub(super) struct TaskHttpForward {
    pub(super) request_id: String,
    pub(super) node_id: String,
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: Vec<RemoteResponseHeader>,
    pub(super) body: Vec<u8>,
}

pub(super) fn task_request(
    request_id: String,
    node_id: String,
    method: &str,
    path: String,
) -> TaskHttpForward {
    TaskHttpForward {
        request_id,
        node_id,
        method: method.to_string(),
        path,
        headers: vec![],
        body: vec![],
    }
}

pub(super) async fn forward_task_http(
    request: TaskHttpForward,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    let forwarded = forward_peer_request(
        &request.node_id,
        &request.method,
        &request.path,
        &request.headers,
        &request.body,
        peers,
        broker,
    )
    .await?;
    Ok(Response::RemoteHttpResponse {
        request_id: request.request_id,
        status: forwarded.status,
        headers: response_headers(forwarded.headers),
        body: forwarded.body,
    })
}

pub(super) async fn forward_peer_request(
    node_id: &str,
    method: &str,
    path: &str,
    headers: &[RemoteResponseHeader],
    body: &[u8],
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<BrokerForwardResponse> {
    let Some(target) = peers.connection_target(node_id) else {
        return Err(anyhow!("unknown Tor peer {node_id}"));
    };
    let headers = headers
        .iter()
        .map(|header| (header.name.clone(), header.value.clone()))
        .collect::<Vec<_>>();
    let response = broker
        .remote_http_exchange(BrokerRemoteHttpRequest {
            endpoint: &target.endpoint,
            node_id: &target.node_id,
            bearer_token: &target.bearer_token,
            method,
            path,
            headers: &headers,
            body,
        })
        .await?;
    mark_auth_rejection(peers, node_id, response.status);
    Ok(response)
}

pub(super) fn response_headers(headers: Vec<(String, String)>) -> Vec<RemoteResponseHeader> {
    headers
        .into_iter()
        .map(|(name, value)| RemoteResponseHeader { name, value })
        .collect()
}

fn mark_auth_rejection(
    peers: &crate::daemon::peer_manager::PeerManager,
    node_id: &str,
    status: u16,
) {
    if matches!(status, 401 | 403) {
        peers.mark_auth_failed(node_id, format!("auth rejected with HTTP {status}"));
    }
}
