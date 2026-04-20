use prost::Message;
use tak_proto::{NodeInfo, NodeStatusResponse};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn node_routes_follow_live_transport_state() {
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            healthy: true,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    context
        .set_transport_state("recovering", Some("self-probe failed"))
        .expect("set recovering");
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let info = handle_remote_v1_request(&context, &store, "GET", "/v1/node/info", None)
        .expect("node info");
    let info = NodeInfo::decode(info.body.as_slice()).expect("decode info");
    assert!(!info.healthy);
    assert_eq!(info.transport_state, "recovering");
    assert_eq!(info.transport_detail, "self-probe failed");

    let status = handle_remote_v1_request(&context, &store, "GET", "/v1/node/status", None)
        .expect("node status");
    let status = NodeStatusResponse::decode(status.body.as_slice()).expect("decode status");
    assert_eq!(status.node.expect("node").transport_state, "recovering");
}
