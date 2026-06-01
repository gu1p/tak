use prost::Message;
use tak_proto::SubmitTaskRequest;
use takd::{
    ForwardRemoteHttpRequest, PeerState, PlaceRemoteRequest, RemoteResponseHeader, Request,
    Response,
};

use crate::support::protocol::send_request;

#[path = "remote_auth_support.rs"]
mod support;

#[tokio::test(flavor = "multi_thread")]
async fn place_remote_refuses_an_auth_failed_peer() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let auth = support::spawn_auth_server(&socket_path, 401).await;

    // The peer rejects the heartbeat with 401, so it is marked auth-failed and is
    // never warm enough to place a task on: the submit is refused rather than
    // forwarded to a peer the bridge cannot authenticate with.
    support::wait_for_peer_state(&auth.peers, PeerState::AuthFailed).await;

    let response = send_request(
        &socket_path,
        &Request::PlaceRemote(PlaceRemoteRequest {
            request_id: "place".into(),
            requirements: Default::default(),
            selection: Default::default(),
            preferred_node_id: None,
            task_run_id: "task-1".into(),
            attempt: 1,
            submit_body: SubmitTaskRequest {
                task_run_id: "task-1".into(),
                ..SubmitTaskRequest::default()
            }
            .encode_to_vec(),
        }),
    )
    .await;

    match response {
        Response::Error { message, .. } => {
            assert!(
                message.contains("auth failed"),
                "unexpected error: {message}"
            )
        }
        other => panic!("expected placement error for auth-failed peer, got {other:?}"),
    }
    assert_eq!(auth.peers.snapshots()[0].state, PeerState::AuthFailed);
    auth.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn forwarded_remote_http_marks_peer_auth_failed_on_403() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let auth = support::spawn_auth_server(&socket_path, 403).await;

    let response = send_request(
        &socket_path,
        &Request::ForwardRemoteHttp(ForwardRemoteHttpRequest {
            request_id: "forward".into(),
            node_id: "builder-auth".into(),
            method: "GET".into(),
            path: "/v1/tasks/task-1/result".into(),
            headers: Vec::<RemoteResponseHeader>::new(),
            body: Vec::new(),
        }),
    )
    .await;

    match response {
        Response::RemoteHttpResponse { status, .. } => assert_eq!(status, 403),
        other => panic!("expected remote HTTP response, got {other:?}"),
    }
    assert_eq!(auth.peers.snapshots()[0].state, PeerState::AuthFailed);
    auth.abort();
}
