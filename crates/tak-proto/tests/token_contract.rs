use tak_proto::{NodeInfo, RemoteTokenPayload, decode_remote_token, encode_remote_token};

#[test]
fn remote_tokens_round_trip_as_binary() {
    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".to_string(),
        node: Some(NodeInfo {
            node_id: "builder-a".to_string(),
            display_name: "Builder A".to_string(),
            base_url: "http://127.0.0.1:43123".to_string(),
            healthy: true,
            pools: vec!["default".to_string()],
            tags: vec!["builder".to_string()],
            capabilities: vec!["linux".to_string()],
            transport: "direct".to_string(),
            transport_state: "ready".to_string(),
            transport_detail: String::new(),
        }),
        bearer_token: "secret-token".to_string(),
    })
    .expect("encode token");

    let decoded = decode_remote_token(&token).expect("decode token");
    let node = decoded.node.expect("node");
    assert_eq!(node.node_id, "builder-a");
    assert_eq!(node.transport_state, "ready");
    assert!(node.transport_detail.is_empty());
}
