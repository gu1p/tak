use super::*;

#[path = "remote/forward.rs"]
mod forward;
#[path = "remote/output.rs"]
mod output;

use forward::{
    TaskHttpForward, forward_peer_request, forward_task_http, response_headers, task_request,
};

pub(super) async fn place_remote_task(
    payload: PlaceRemoteRequest,
    peer: crate::daemon::peer_manager::PeerSnapshot,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
    tasks: &DaemonTaskHandles,
) -> Result<Response> {
    let request_id = payload.request_id;
    let task_handle = tasks.register(&peer.node_id, &payload.task_run_id)?;
    let forwarded = forward_peer_request(
        &peer.node_id,
        "POST",
        "/v1/tasks/submit",
        &[],
        &payload.submit_body,
        peers,
        broker,
    )
    .await
    .map_err(|err| anyhow!("remote node {} unavailable: {err:#}", peer.node_id))?;
    Ok(Response::RemotePlaced {
        request_id,
        task_handle,
        peer: Box::new(peer),
        status: forwarded.status,
        headers: response_headers(forwarded.headers),
        body: forwarded.body,
    })
}

pub(super) async fn forward_remote_http(
    payload: ForwardRemoteHttpRequest,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    forward_task_http(
        TaskHttpForward {
            request_id: payload.request_id,
            node_id: payload.node_id,
            method: payload.method,
            path: payload.path,
            headers: payload.headers,
            body: payload.body,
        },
        peers,
        broker,
    )
    .await
}

pub(super) async fn stream_task_events(
    request_id: String,
    task: crate::daemon::protocol::daemon_tasks::DaemonTaskHandle,
    payload: StreamTaskEventsRequest,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    let path = format!(
        "/v1/tasks/{}/events?after_seq={}",
        task.task_run_id, payload.after_seq
    );
    forward_task_http(
        task_request(request_id, task.node_id, "GET", path),
        peers,
        broker,
    )
    .await
}

pub(super) async fn cancel_task(
    request_id: String,
    task: crate::daemon::protocol::daemon_tasks::DaemonTaskHandle,
    payload: CancelTaskRequest,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    let path = format!(
        "/v1/tasks/{}/cancel?attempt={}",
        task.task_run_id, payload.attempt
    );
    forward_task_http(
        task_request(request_id, task.node_id, "POST", path),
        peers,
        broker,
    )
    .await
}

pub(super) async fn get_task_result(
    request_id: String,
    task: crate::daemon::protocol::daemon_tasks::DaemonTaskHandle,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    let path = format!("/v1/tasks/{}/result", task.task_run_id);
    forward_task_http(
        task_request(request_id, task.node_id, "GET", path),
        peers,
        broker,
    )
    .await
}

pub(super) async fn get_output_range(
    request_id: String,
    task: crate::daemon::protocol::daemon_tasks::DaemonTaskHandle,
    payload: GetOutputRangeRequest,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    output::get_output_range(request_id, task, payload, peers, broker).await
}
