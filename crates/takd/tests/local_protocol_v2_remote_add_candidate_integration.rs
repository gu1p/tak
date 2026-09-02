#![cfg(unix)]

use crate::support::raw_local_protocol::RawLocalProtocol;
use tak_core::v2::RemoteRequirements;
use tak_proto::local_daemon::v2::{Operation, Request, Response, decode_response, encode_request};

mod worker;

#[tokio::test]
async fn added_remote_is_immediately_available_as_a_daemon_candidate() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server_url = base_url.clone();
    let worker = tokio::spawn(async move { worker::serve(listener, server_url).await });
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;
    let add = Request {
        request_id: "add".into(),
        operation: Operation::AddRemote {
            invite: worker::direct_invite(&base_url),
        },
    };
    let added = daemon.exchange(&encode_request(&add).unwrap()).await;
    assert!(matches!(
        decode_response(added.trim().as_bytes(), "add").unwrap(),
        Response::RemoteAdded { .. }
    ));
    let resolve = Request {
        request_id: "resolve".into(),
        operation: Operation::ResolveRemoteCandidates {
            requirements: RemoteRequirements {
                pool: Some("build".into()),
                required_tags: vec!["linux".into()],
                required_capabilities: vec!["docker".into()],
                transport: Some("direct".into()),
            },
        },
    };
    let resolved = daemon.exchange(&encode_request(&resolve).unwrap()).await;
    let Response::RemoteCandidates { candidates, .. } =
        decode_response(resolved.trim().as_bytes(), "resolve").unwrap()
    else {
        panic!("expected remote candidates response")
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].node_id, "builder-a");
    worker.await.unwrap();
}

#[tokio::test]
async fn transient_snapshot_failure_during_add_does_not_break_immediate_candidate_resolution() {
    let root = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server_url = base_url.clone();
    let worker = tokio::spawn(async move {
        worker::serve_with_transient_snapshot_failure(listener, server_url).await
    });
    let mut daemon =
        RawLocalProtocol::start_with_remote_inventory(root.path(), takd::TorBroker::new()).await;
    let add = Request {
        request_id: "add-after-transient-snapshot-failure".into(),
        operation: Operation::AddRemote {
            invite: worker::direct_invite(&base_url),
        },
    };
    let added = daemon.exchange(&encode_request(&add).unwrap()).await;
    assert!(matches!(
        decode_response(
            added.trim().as_bytes(),
            "add-after-transient-snapshot-failure"
        )
        .unwrap(),
        Response::RemoteAdded { .. }
    ));

    let resolve = Request {
        request_id: "resolve-after-transient-snapshot-failure".into(),
        operation: Operation::ResolveRemoteCandidates {
            requirements: RemoteRequirements {
                pool: Some("build".into()),
                required_tags: vec!["linux".into()],
                required_capabilities: vec!["docker".into()],
                transport: Some("direct".into()),
            },
        },
    };
    let resolved = daemon.exchange(&encode_request(&resolve).unwrap()).await;
    let Response::RemoteCandidates { candidates, .. } = decode_response(
        resolved.trim().as_bytes(),
        "resolve-after-transient-snapshot-failure",
    )
    .unwrap() else {
        panic!("expected remote candidates response")
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].node_id, "builder-a");
    worker.await.unwrap();
}
