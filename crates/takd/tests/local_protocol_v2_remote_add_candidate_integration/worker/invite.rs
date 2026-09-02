use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};

pub(in super::super) fn direct_invite(base_url: &str) -> String {
    encode_remote_token(&RemoteTokenPayload {
        version: "v2".into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "Builder A".into(),
            base_url: base_url.into(),
            healthy: true,
            pools: vec![],
            tags: vec![],
            capabilities: vec![],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        bearer_token: "secret".into(),
    })
    .unwrap()
}
