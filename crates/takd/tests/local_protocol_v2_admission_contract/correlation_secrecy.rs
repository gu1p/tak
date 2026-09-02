use crate::support::raw_local_protocol::RawLocalProtocol;

#[tokio::test(flavor = "multi_thread")]
async fn protocol_errors_correlate_only_safe_ids_and_never_echo_request_secrets() {
    let mut daemon = RawLocalProtocol::start().await;
    let secret = "TAK_SECRET_SENTINEL";
    let strict_invalid = format!(
        r#"{{"protocol_version":2,"credential":"{secret}","request_id":"safe-id","operation":{{"type":"ListRuns"}}}}"#
    );
    let response = daemon.exchange(&strict_invalid).await;
    super::assert_json_response(&response, invalid_response(serde_json::json!("safe-id")));
    assert!(!response.contains(secret));

    let unsafe_ids = [
        format!(
            r#"{{"protocol_version":2,"request_id":"first","request\u005fid":"{secret}","operation":{{"type":"ListRuns"}}}}"#
        ),
        format!(
            r#"{{"protocol_version":2,"request_id":"{}","operation":{{"type":"ListRuns"}}}}"#,
            "x".repeat(129)
        ),
    ];
    for request in unsafe_ids {
        let response = daemon.exchange(&request).await;
        super::assert_json_response(&response, invalid_response(serde_json::Value::Null));
        assert!(!response.contains(secret));
        assert!(!response.contains(&"x".repeat(129)));
    }

    let special_id = "quote\"\\slash";
    let request = format!(
        r#"{{"protocol_version":2,"credential":"{secret}","request_id":{},"operation":{{"type":"ListRuns"}}}}"#,
        serde_json::to_string(special_id).expect("encode id")
    );
    let response = daemon.exchange(&request).await;
    super::assert_json_response(&response, invalid_response(serde_json::json!(special_id)));

    let malformed_legacy =
        format!(r#"{{"type":"Status","request_id":"legacy","credential":"{secret}""#);
    let response = daemon.exchange(&malformed_legacy).await;
    super::assert_json_response(
        &response,
        serde_json::json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": null,
            "message": "This takd requires protocol v2. Upgrade tak, takd, and workers together.",
            "code": "protocol_version_unsupported",
            "retryable": false
        }),
    );
    assert!(!response.contains(secret));

    let valid_legacy_error =
        format!(r#"{{"type":"NoSuchRequest","request_id":"legacy","credential":"{secret}"}}"#);
    let response = daemon.exchange(&valid_legacy_error).await;
    super::assert_json_response(
        &response,
        serde_json::json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": "legacy",
            "message": "This takd requires protocol v2. Upgrade tak, takd, and workers together.",
            "code": "protocol_version_unsupported",
            "retryable": false
        }),
    );
    assert!(!response.contains(secret));
}

fn invalid_response(request_id: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": 2,
        "type": "Error",
        "request_id": request_id,
        "message": "Invalid protocol v2 request.",
        "code": "protocol_request_invalid",
        "retryable": false
    })
}
