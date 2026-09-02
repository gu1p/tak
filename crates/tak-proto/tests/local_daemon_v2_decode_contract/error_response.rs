use tak_proto::local_daemon::v2::{ErrorResponse, RequestDecodeError, RequestDecodeErrorCode};

#[test]
fn shared_error_response_owns_stable_v2_wire_codes_and_safe_correlation() {
    let inactive = ErrorResponse::v2_not_active("list".to_string());
    assert_eq!(
        serde_json::to_string(&inactive).expect("encode response"),
        r#"{"protocol_version":2,"type":"Error","request_id":"list","message":"Protocol v2 run operations are not active in this takd build. Upgrade tak, takd, and workers together.","code":"protocol_v2_not_active","retryable":false}"#
    );

    let invalid = ErrorResponse::from(RequestDecodeError {
        code: RequestDecodeErrorCode::RequestInvalid,
        request_id: None,
    });
    assert_eq!(
        serde_json::to_value(invalid).expect("encode response"),
        serde_json::json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": null,
            "message": "Invalid protocol v2 request.",
            "code": "protocol_request_invalid",
            "retryable": false
        })
    );
}

#[test]
fn unsupported_remote_invites_have_redacted_upgrade_guidance() {
    let response = ErrorResponse::remote_invite_unsupported("add".into());
    let value = serde_json::to_value(response).expect("encode response");
    assert_eq!(value["code"], "remote_invite_unsupported");
    assert!(
        value["message"]
            .as_str()
            .unwrap()
            .contains("upgrade tak, takd, and workers together")
    );
    assert!(!value.to_string().contains("secret"));
}
