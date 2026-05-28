use prost::Message;
use std::time::Duration;
use tak_proto::NodeInfo;
use takd::{TorBroker, new_shared_manager, run_server_with_broker};

use crate::support;
use support::local_broker_http::{response_body, send_broker_get_h2};

#[path = "local_tor_broker_http2_session_contract/heartbeat_reuse.rs"]
mod heartbeat_reuse;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_reuses_one_http2_connection_for_repeated_requests() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = support::http2_remote::Http2Remote::spawn(node_info_body()).await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    let first = send_broker_get_h2(&socket_path, "builder-h2").await;
    let second = send_broker_get_h2(&socket_path, "builder-h2").await;

    assert_eq!(
        NodeInfo::decode(response_body(&first))
            .expect("first node")
            .node_id,
        "builder-h2"
    );
    assert_eq!(
        NodeInfo::decode(response_body(&second))
            .expect("second node")
            .node_id,
        "builder-h2"
    );
    assert_eq!(remote.connection_count(), 1);
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_multiplexes_parallel_http2_requests_on_one_connection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = support::http2_remote::Http2Remote::spawn_delayed(
        node_info_body(),
        Duration::from_millis(150),
    )
    .await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    let _ = send_broker_get_h2(&socket_path, "builder-h2").await;
    let (first, second) = tokio::join!(
        send_broker_get_h2(&socket_path, "builder-h2"),
        send_broker_get_h2(&socket_path, "builder-h2")
    );

    assert_eq!(
        NodeInfo::decode(response_body(&first))
            .expect("first node")
            .node_id,
        "builder-h2"
    );
    assert_eq!(
        NodeInfo::decode(response_body(&second))
            .expect("second node")
            .node_id,
        "builder-h2"
    );
    assert_eq!(remote.connection_count(), 1);
    assert_eq!(remote.max_in_flight(), 2);
    server.abort();
}

fn node_info_body() -> Vec<u8> {
    NodeInfo {
        node_id: "builder-h2".into(),
        display_name: "builder-h2".into(),
        base_url: "http://builder-h2.onion".into(),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "tor".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
    .encode_to_vec()
}
