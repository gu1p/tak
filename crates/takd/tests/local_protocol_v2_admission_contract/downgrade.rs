use takd::Request;

use crate::support::protocol::acquire_request;
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

    assert_eq!(
        daemon
            .exchange(r#"{"type":"Status","request_id":"status"}"#)
            .await,
        concat!(
            r#"{"type":"StatusSnapshot","request_id":"status","status":{"active_leases":0,"pending_requests":0,"usage":[]}}"#,
            "\n"
        )
    );
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
    serde_json::to_string(&Request::AcquireLease(acquire_request(request_id)))
        .expect("encode legacy request")
}
