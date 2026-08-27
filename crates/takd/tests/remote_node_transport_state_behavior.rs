use prost::Message;
use tak_proto::{ErrorResponse, NodeInfo, NodeStatusResponse};
use takd::{RemoteNodeContext, SubmitAttemptStore};

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
        crate::support::runtime_config::isolated(),
    );
    context
        .set_transport_state("recovering", Some("self-probe failed"))
        .expect("set recovering");
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let info = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/node/info",
        &[],
        None,
    )
    .expect("node info");
    let info = NodeInfo::decode(info.body.as_slice()).expect("decode info");
    assert!(!info.healthy);
    assert_eq!(info.transport_state, "recovering");
    assert_eq!(info.transport_detail, "self-probe failed");

    let status = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/node/status",
        &[],
        None,
    )
    .expect("node status");
    let status = NodeStatusResponse::decode(status.body.as_slice()).expect("decode status");
    assert_eq!(status.node.expect("node").transport_state, "recovering");

    context
        .set_transport_state("pending", Some("starting onion service"))
        .expect("set pending");
    let submit = crate::support::remote_v1_http_submit::submit_request("pending-work", Vec::new());
    let response = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        &[],
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    assert_eq!(response.status_code, 503);
    let error = ErrorResponse::decode(response.body.as_slice()).expect("decode submit error");
    assert_eq!(error.message, "transport_not_ready");
}
