use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test(flavor = "multi_thread")]
async fn v2_and_legacy_hybrids_never_fall_back_to_legacy_dispatch() {
    let mut daemon = RawLocalProtocol::start().await;
    let exact_v2 = legacy_acquire("exact-v2").replacen('{', r#"{"protocol_version":2,"#, 1);
    super::assert_json_response(
        &daemon.exchange(&exact_v2).await,
        error(
            "exact-v2",
            "Invalid protocol v2 request.",
            "protocol_request_invalid",
        ),
    );

    let legacy = legacy_acquire("missing-version");
    let versionless_intent = format!(
        r#"{},"operation":{{"type":"ListRuns"}}}}"#,
        legacy.strip_suffix('}').expect("legacy object")
    );
    super::assert_json_response(
        &daemon.exchange(&versionless_intent).await,
        error(
            "missing-version",
            "protocol_version must appear exactly once as the integer 2.",
            "protocol_version_invalid",
        ),
    );

    let huge_version = r#"{"protocol_version":1e999999,"type":"Status","request_id":"huge"}"#;
    super::assert_json_response(
        &daemon.exchange(huge_version).await,
        error(
            "huge",
            "protocol_version must appear exactly once as the integer 2.",
            "protocol_version_invalid",
        ),
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn every_versionless_local_v1_operation_is_rejected_with_upgrade_guidance() {
    let mut daemon = RawLocalProtocol::start().await;
    let requests = [
        r#"{"type":"AcquireLease","request_id":"acquire","client":{"user":"alice","pid":7,"session_id":"s"},"task":{"label":"//:check","attempt":1},"needs":[],"ttl_ms":30000}"#,
        r#"{"type":"RenewLease","request_id":"renew","lease_id":"lease-1","ttl_ms":30000}"#,
        r#"{"type":"ReleaseLease","request_id":"release","lease_id":"lease-1"}"#,
        r#"{"type":"Status","request_id":"status"}"#,
        r#"{"type":"PeersList","request_id":"peers"}"#,
    ];

    for request in requests {
        let request_id = serde_json::from_str::<serde_json::Value>(request).unwrap()["request_id"]
            .as_str()
            .unwrap()
            .to_string();
        super::assert_json_response(
            &daemon.exchange(request).await,
            error(
                &request_id,
                "This takd requires protocol v2. Upgrade tak, takd, and workers together.",
                "protocol_version_unsupported",
            ),
        );
    }
}

fn error(request_id: &str, message: &str, code: &str) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": 2,
        "type": "Error",
        "request_id": request_id,
        "message": message,
        "code": code,
        "retryable": false
    })
}

fn legacy_acquire(request_id: &str) -> String {
    format!(
        r#"{{"type":"AcquireLease","request_id":"{request_id}","client":{{"user":"alice","pid":7,"session_id":"s"}},"task":{{"label":"//:check","attempt":1}},"needs":[],"ttl_ms":30000}}"#
    )
}
