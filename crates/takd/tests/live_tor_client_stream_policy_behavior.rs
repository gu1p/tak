use tak_proto::NodeInfo;
use takd::{RemoteNodeContext, service::observe_live_tor_client_stream};

#[test]
fn live_tor_client_stream_observation_does_not_clear_recovering_state() {
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            healthy: false,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "recovering".into(),
            transport_detail: "self-probe failed".into(),
        },
        "secret".into(),
    );

    observe_live_tor_client_stream(&context);

    let node = context.node_info().expect("node info");
    assert!(!node.healthy);
    assert_eq!(node.transport_state, "recovering");
    assert_eq!(node.transport_detail, "self-probe failed");
}
