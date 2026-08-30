use tak_proto::local_daemon::v2::{
    DaemonErrorCode, ErrorResponse, MAX_ERROR_RESPONSE_FRAME_BYTES, RequestDecodeError,
    RequestDecodeErrorCode, ResponseDecodeError, decode_error_response,
};

#[test]
fn decoder_returns_only_a_fixed_code_and_discards_the_daemon_message() {
    let secret = "DAEMON_MESSAGE_SECRET_MUST_NOT_ESCAPE";
    let raw = format!(
        r#"{{"protocol_version":2,"type":"Error","request_id":"expected","message":"{secret}","code":"protocol_v2_not_active","retryable":false}}"#
    );
    let code = decode_error_response(raw.as_bytes(), "expected").expect("strict error response");
    assert_eq!(code, DaemonErrorCode::ProtocolV2NotActive);
    assert!(!format!("{code:?}").contains(secret));
}

#[test]
fn decoder_stays_compatible_with_daemon_owned_error_encoding() {
    let inactive = serde_json::to_vec(&ErrorResponse::v2_not_active("expected".into()))
        .expect("encode daemon response");
    assert_eq!(
        decode_error_response(&inactive, "expected"),
        Ok(DaemonErrorCode::ProtocolV2NotActive)
    );

    let admission_codes = [
        (
            RequestDecodeErrorCode::VersionInvalid,
            DaemonErrorCode::ProtocolVersionInvalid,
        ),
        (
            RequestDecodeErrorCode::VersionUnsupported,
            DaemonErrorCode::ProtocolVersionUnsupported,
        ),
        (
            RequestDecodeErrorCode::RequestInvalid,
            DaemonErrorCode::ProtocolRequestInvalid,
        ),
    ];
    for (request_code, expected) in admission_codes {
        let response = ErrorResponse::from(RequestDecodeError {
            code: request_code,
            request_id: Some("expected".into()),
        });
        let response = serde_json::to_vec(&response).expect("encode daemon rejection");
        assert_eq!(decode_error_response(&response, "expected"), Ok(expected));
    }
}

#[test]
fn decoder_enforces_the_payload_byte_cap_before_parsing() {
    assert_eq!(MAX_ERROR_RESPONSE_FRAME_BYTES, 64 * 1024);
    let at_limit = response_with_size(MAX_ERROR_RESPONSE_FRAME_BYTES);
    assert_eq!(
        decode_error_response(&at_limit, "expected"),
        Ok(DaemonErrorCode::ProtocolV2NotActive)
    );
    let over_limit = response_with_size(MAX_ERROR_RESPONSE_FRAME_BYTES + 1);
    assert_eq!(
        decode_error_response(&over_limit, "expected"),
        Err(ResponseDecodeError::FrameTooLarge)
    );
}

fn response_with_size(size: usize) -> Vec<u8> {
    let prefix = br#"{"protocol_version":2,"type":"Error","request_id":"expected","message":""#;
    let suffix = br#"","code":"protocol_v2_not_active","retryable":false}"#;
    let padding = size - prefix.len() - suffix.len();
    [prefix.as_slice(), &vec![b'x'; padding], suffix.as_slice()].concat()
}
