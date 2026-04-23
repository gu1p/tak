use takd::{ReleaseLeaseRequest, RenewLeaseRequest, Request, Response};

use crate::support;

use support::protocol::send_request;
use support::protocol_server::seeded_protocol_server;

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_renew_lease_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let (server, lease_id) = seeded_protocol_server(
        temp.path().join("state/takd.sqlite"),
        socket_path.clone(),
        "renew-seed",
    );

    let renewed = send_request(
        &socket_path,
        &Request::RenewLease(RenewLeaseRequest {
            request_id: "renew".into(),
            lease_id,
            ttl_ms: 15_000,
        }),
    )
    .await;

    assert!(matches!(
        renewed,
        Response::LeaseRenewed { ttl_ms: 15_000, .. }
    ));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_release_lease_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let (server, lease_id) = seeded_protocol_server(
        temp.path().join("state/takd.sqlite"),
        socket_path.clone(),
        "release-seed",
    );

    let released = send_request(
        &socket_path,
        &Request::ReleaseLease(ReleaseLeaseRequest {
            request_id: "release".into(),
            lease_id,
        }),
    )
    .await;

    assert!(matches!(released, Response::LeaseReleased { .. }));
    server.abort();
}
