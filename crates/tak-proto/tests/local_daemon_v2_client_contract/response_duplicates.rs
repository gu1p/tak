use tak_proto::local_daemon::v2::{ResponseDecodeError, decode_error_response};

#[test]
fn decoder_rejects_duplicate_response_fields_including_escaped_keys() {
    let cases = [
        r#"{"protocol_version":2,"protocol_version":2,"type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"protocol\u005fversion":2,"type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"type":"Error","type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","message":"ok","message":"ok","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","code":"protocol_v2_not_active","retryable":false}"#,
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false,"retryable":false}"#,
    ];

    for raw in cases {
        assert_eq!(
            decode_error_response(raw.as_bytes(), "expected"),
            Err(ResponseDecodeError::ProtocolMismatch),
            "{raw}"
        );
    }
}
