use tak_proto::NodeInfo;
use tak_proto::worker_v2::decode_snapshot;
use takd::{RemoteNodeContext, SubmitAttemptStore};

#[test]
fn worker_snapshot_route_requires_protocol_v2_and_returns_typed_capacity() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "worker-a".into(),
            display_name: "worker-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        crate::support::runtime_config::builder()
            .with_skip_exec_root_probe(true)
            .build(),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let missing = takd::daemon::remote::handle_worker_http_request(
        &context,
        &store,
        "GET",
        "/v2/worker/snapshot",
        &[],
        None,
    )
    .unwrap();
    assert_eq!(missing.status_code, 426);
    assert!(String::from_utf8_lossy(&missing.body).contains("upgrade tak, takd, and workers"));

    let headers = [("x-tak-protocol-version".into(), "v2".into())];
    let response = takd::daemon::remote::handle_worker_http_request(
        &context,
        &store,
        "GET",
        "/v2/worker/snapshot",
        &headers,
        None,
    )
    .unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, "application/json");
    let snapshot = decode_snapshot(&response.body).unwrap();
    assert_eq!(snapshot.node_id, "worker-a");
    assert!(snapshot.capacity.execution_slots > 0);
}
