use takd::WorkerConnectionTarget;

use crate::support::http2_remote::Http2Remote;

#[path = "connection_keeper_contract/support.rs"]
mod keeper;

use keeper::{ENDPOINT, NODE, peers, ping_body, wait_for_connections};

#[tokio::test(flavor = "multi_thread")]
async fn keeper_eagerly_holds_a_warm_connection_without_any_submit() {
    let remote = Http2Remote::spawn(ping_body()).await;
    let broker = crate::support::local_runtime::tor_broker(remote.addr.clone());
    // No heartbeat and no submit: the keeper alone must open the connection.
    peers().spawn_connection_keeper(broker.clone());

    wait_for_connections(&remote, 1).await;
    assert!(remote.connection_count() >= 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn keeper_redials_immediately_after_the_connection_is_lost() {
    let remote = Http2Remote::spawn(ping_body()).await;
    let broker = crate::support::local_runtime::tor_broker(remote.addr.clone());
    peers().spawn_connection_keeper(broker.clone());
    wait_for_connections(&remote, 1).await;

    // Simulate a lost link by evicting the pooled session; the keeper must
    // re-establish it on its next tick. (Real silent-transport loss is detected
    // by hyper keep-alive and exercised end to end by the live-Tor example.)
    broker
        .evict_http2_session_for_peer(ENDPOINT, NODE, "secret")
        .await;

    wait_for_connections(&remote, 2).await;
    assert!(remote.connection_count() >= 2);

    // The redialed connection must actually work, not merely accept a TCP socket.
    let response = broker
        .worker_v2_http_exchange(
            &WorkerConnectionTarget {
                node_id: NODE.into(),
                endpoint: ENDPOINT.into(),
                bearer_token: "secret".into(),
                transport: "tor".into(),
            },
            "GET",
            "/v2/worker/ping",
            &[],
        )
        .await
        .expect("redialed warm connection serves a request");
    assert_eq!(response.status, 200);
}
