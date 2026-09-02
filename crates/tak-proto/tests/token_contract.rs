use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use prost::Message;
use tak_proto::{NodeInfo, RemoteTokenPayload, decode_remote_token, encode_remote_token};

#[test]
fn direct_v2_tokens_round_trip_with_an_explicit_v2_prefix() {
    let token = encode_remote_token(&payload("v2")).expect("encode token");

    assert!(token.starts_with("takd:v2:"), "{token}");
    let decoded = decode_remote_token(&token).expect("decode token");
    assert_eq!(decoded.version, "v2");
    assert_eq!(decoded.node.expect("node").node_id, "builder-a");
}

#[test]
fn legacy_v1_tokens_are_rejected_before_payload_decoding() {
    for token in [
        "takd:v1:not-base64".to_string(),
        raw_token("takd:v1:", &payload("v1")),
    ] {
        let error = decode_remote_token(&token).expect_err("reject legacy token");
        let message = error.to_string();
        assert!(
            message.contains("upgrade tak, takd, and workers together"),
            "{message}"
        );
        assert!(!message.contains("base64"), "{message}");
        assert!(!message.contains("protobuf"), "{message}");
    }
}

#[test]
fn direct_token_prefix_and_payload_versions_must_match() {
    let token = raw_token("takd:v2:", &payload("v1"));

    let error = decode_remote_token(&token).expect_err("reject mismatched token");

    assert!(
        error
            .to_string()
            .contains("direct invite prefix `takd:v2:` requires payload version `v2`, got `v1`"),
        "{error:#}"
    );
}

fn raw_token(prefix: &str, payload: &RemoteTokenPayload) -> String {
    format!(
        "{prefix}{}",
        URL_SAFE_NO_PAD.encode(payload.encode_to_vec())
    )
}

fn payload(version: &str) -> RemoteTokenPayload {
    RemoteTokenPayload {
        version: version.into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "Builder A".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        bearer_token: "secret-token".into(),
    }
}
