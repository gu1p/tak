#![allow(clippy::await_holding_lock)]

use takd::{ReleaseLeaseRequest, RenewLeaseRequest, Request, Response, StatusRequest, run_daemon};

mod support;

use support::env::{EnvGuard, env_lock};
use support::http::wait_for_node_info;
use support::protocol::{acquire_request, free_bind_addr, send_request};

#[tokio::test(flavor = "multi_thread")]
async fn run_daemon_serves_protocol_and_remote_v1_http() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let bind_addr = free_bind_addr();
    env.set(
        "TAKD_DB_PATH",
        temp.path().join("state/takd.sqlite").display().to_string(),
    );
    env.set("TAKD_REMOTE_V1_BIND_ADDR", bind_addr.clone());
    env.set("TAKD_NODE_ID", "daemon-direct");
    env.set("TAKD_DISPLAY_NAME", "daemon-direct");
    env.set("TAKD_BEARER_TOKEN", "secret");
    env.set("TAKD_NODE_TRANSPORT", "direct");

    let socket_for_task = socket_path.clone();
    let daemon = tokio::spawn(async move { run_daemon(&socket_for_task).await });
    let node = wait_for_node_info(&bind_addr, &bind_addr, "secret").await;
    assert_eq!(node.node_id, "daemon-direct");
    assert_eq!(node.transport, "direct");

    let status = send_request(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "s".into(),
        }),
    )
    .await;
    assert!(matches!(status, Response::StatusSnapshot { .. }));

    let granted = send_request(
        &socket_path,
        &Request::AcquireLease(acquire_request("acquire")),
    )
    .await;
    let lease_id = match granted {
        Response::LeaseGranted { lease, .. } => lease.lease_id,
        other => panic!("expected lease grant, got {other:?}"),
    };
    assert!(matches!(
        send_request(
            &socket_path,
            &Request::RenewLease(RenewLeaseRequest {
                request_id: "renew".into(),
                lease_id: lease_id.clone(),
                ttl_ms: 15_000,
            })
        )
        .await,
        Response::LeaseRenewed { ttl_ms: 15_000, .. }
    ));
    assert!(matches!(
        send_request(
            &socket_path,
            &Request::ReleaseLease(ReleaseLeaseRequest {
                request_id: "release".into(),
                lease_id,
            })
        )
        .await,
        Response::LeaseReleased { .. }
    ));
    daemon.abort();
}
