use tak_proto::local_daemon::v2::{ResponseDecodeError, decode_error_response};

#[test]
fn decoder_rejects_every_untrusted_or_uncorrelated_response_without_reflection() {
    let secret = "UNTRUSTED_RESPONSE_SECRET_SENTINEL";
    assert_eq!(
        decode_error_response(&[0xff], "expected"),
        Err(ResponseDecodeError::ProtocolMismatch)
    );
    let cases = [
        String::new(),
        "{".to_string(),
        "[]".to_string(),
        error("1", "expected", "false", "protocol_v2_not_active"),
        error("3", "expected", "false", "protocol_v2_not_active"),
        error("null", "expected", "false", "protocol_v2_not_active"),
        error(r#""2""#, "expected", "false", "protocol_v2_not_active"),
        error("2.0", "expected", "false", "protocol_v2_not_active"),
        r#"{"type":"Error","request_id":"expected","message":"legacy","code":"protocol_v2_not_active","retryable":false}"#.into(),
        error("2", secret, "false", "protocol_v2_not_active"),
        error("2", "expected", "true", "protocol_v2_not_active"),
        error("2", "expected", "false", "unknown_code"),
        r#"{"protocol_version":2,"type":"Error","request_id":null,"message":"ok","code":"protocol_v2_not_active","retryable":false}"#.into(),
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","code":"protocol_v2_not_active","retryable":false}"#.into(),
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","message":null,"code":"protocol_v2_not_active","retryable":false}"#.into(),
        r#"{"protocol_version":2,"type":"Error","request_id":"expected","message":42,"code":"protocol_v2_not_active","retryable":false}"#.into(),
        format!(
            r#"{{"protocol_version":2,"type":"Error","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false,"extra":"{secret}"}}"#
        ),
        r#"{"protocol_version":2,"type":"RunList","request_id":"expected","runs":[]}"#.into(),
        r#"{"protocol_version":2,"type":"RunList","request_id":"expected","message":"ok","code":"protocol_v2_not_active","retryable":false}"#.into(),
        format!(
            "{} {{}}",
            error("2", "expected", "false", "protocol_v2_not_active")
        ),
        format!(
            "{} ",
            error("2", "expected", "false", "protocol_v2_not_active")
        ),
        format!(
            "{}\n",
            error("2", "expected", "false", "protocol_v2_not_active")
        ),
    ];

    for raw in cases {
        let rejected = decode_error_response(raw.as_bytes(), "expected")
            .expect_err("untrusted response must be rejected");
        assert_eq!(rejected, ResponseDecodeError::ProtocolMismatch, "{raw}");
        assert!(!format!("{rejected:?}").contains(secret));
    }

    let invalid_id = "line\nbreak";
    let raw = error("2", invalid_id, "false", "protocol_v2_not_active");
    assert_eq!(
        decode_error_response(raw.as_bytes(), invalid_id),
        Err(ResponseDecodeError::ProtocolMismatch)
    );
}

fn error(version: &str, request_id: &str, retryable: &str, code: &str) -> String {
    let request_id = serde_json::to_string(request_id).expect("encode request id");
    format!(
        r#"{{"protocol_version":{version},"type":"Error","request_id":{request_id},"message":"message","code":"{code}","retryable":{retryable}}}"#
    )
}
