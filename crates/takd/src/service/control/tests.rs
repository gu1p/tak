#![cfg(test)]

use tak_proto::NodeInfo;

use super::AgentControlState;
use crate::daemon::remote::{RemoteNodeContext, RemoteRuntimeConfig};

#[test]
fn replacing_the_same_tor_node_preserves_its_remote_work_context() {
    let state = AgentControlState::default();
    let first = state
        .set_context(context("http://first.onion", "ready"))
        .expect("install first context");
    assert!(first.claim_remote_runtime_services());
    first
        .register_active_execution("accepted:1".into(), "accepted", 1)
        .expect("register accepted work");

    let second = state
        .set_context(context("http://second.onion", "pending"))
        .expect("replace transport context");

    assert_eq!(second.active_execution_keys().unwrap(), ["accepted:1"]);
    assert!(
        !second.claim_remote_runtime_services(),
        "recovery must retain the first context's runtime-service ownership"
    );
    let node = second.node_info().expect("updated node");
    assert_eq!(node.base_url, "http://second.onion");
    assert_eq!(node.transport_state, "pending");

    state
        .mark_transport_recovering("onion session restarting")
        .expect("mark transport recovering");
    let node = second.node_info().expect("recovering node");
    assert_eq!(node.transport_state, "recovering");
    assert_eq!(node.transport_detail, "onion session restarting");
}

fn context(base_url: &str, transport_state: &str) -> RemoteNodeContext {
    RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            base_url: base_url.into(),
            transport: "tor".into(),
            transport_state: transport_state.into(),
            ..NodeInfo::default()
        },
        "secret".into(),
        RemoteRuntimeConfig::isolated_for_test(),
    )
}
