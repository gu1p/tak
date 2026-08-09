#![cfg(test)]

use tak_proto::NodeInfo;

use crate::daemon::remote::{RemoteNodeContext, RemoteRuntimeConfig};

use super::handle_accepted_stream_side_effects;

#[test]
fn accepted_stream_does_not_clear_recovering_state() {
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
        RemoteRuntimeConfig::isolated_for_test(),
    );

    handle_accepted_stream_side_effects(&context);

    let node = context.node_info().expect("node info");
    assert!(!node.healthy);
    assert_eq!(node.transport_state, "recovering");
    assert_eq!(node.transport_detail, "self-probe failed");
}
