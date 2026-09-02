use tak_proto::local_daemon::v2::{RequestDecodeError, RequestDecodeErrorCode, decode_request};

#[test]
fn every_versionless_legacy_frame_is_rejected_as_an_unsupported_protocol() {
    let legacy = r#"{"type":"Status","request_id":"legacy"}"#;
    let error = assert_code(legacy, RequestDecodeErrorCode::VersionUnsupported);
    assert_eq!(error.request_id.as_deref(), Some("legacy"));

    let malformed_legacy = r#"{"type":"Status","credential":"secret""#;
    let error = assert_code(malformed_legacy, RequestDecodeErrorCode::VersionUnsupported);
    assert_eq!(error.request_id, None);

    let nested_marker = r#"{"type":"Status","request_id":"legacy","metadata":{"protocol_version":2,"operation":true}}"#;
    let error = assert_code(nested_marker, RequestDecodeErrorCode::VersionUnsupported);
    assert_eq!(error.request_id.as_deref(), Some("legacy"));

    let versionless_v2 = r#"{"type":"Status","oper\u0061tion":{"type":"ListRuns"}}"#;
    assert_code(versionless_v2, RequestDecodeErrorCode::VersionInvalid);
}

#[test]
fn explicit_duplicate_invalid_and_unsupported_versions_are_rejected() {
    let invalid = [
        r#"{"protocol_version":2,"protocol_version":2,"request_id":"dup","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":2,"protocol\u005fversion":2,"request_id":"dup","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":null,"request_id":"null","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":-2,"request_id":"negative","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":2.0,"request_id":"float","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":"2","request_id":"string","operation":{"type":"ListRuns"}}"#,
        r#"{"protocol_version":1e999999,"request_id":"huge","operation":{"type":"ListRuns"}}"#,
    ];
    for raw in invalid {
        assert_code(raw, RequestDecodeErrorCode::VersionInvalid);
    }

    for version in [0, 1, 3] {
        let raw = format!(
            r#"{{"protocol_version":{version},"request_id":"unsupported","operation":{{"type":"ListRuns"}}}}"#
        );
        assert_code(&raw, RequestDecodeErrorCode::VersionUnsupported);
    }

    let truncated =
        r#"{"protocol_version":2,"request_id":"partial","operation":{"type":"ListRuns"}"#;
    assert_code(truncated, RequestDecodeErrorCode::RequestInvalid);
}

#[test]
fn probe_preserves_v2_intent_without_converting_unknown_or_huge_values() {
    let deep_value = format!("{}0{}", "[".repeat(256), "]".repeat(256));
    let deep_before_version = format!(
        r#"{{"padding":{deep_value},"protocol_version":2,"request_id":"safe","operation":{{"type":"ListRuns"}}}}"#
    );
    let error = assert_code(&deep_before_version, RequestDecodeErrorCode::RequestInvalid);
    assert_eq!(error.request_id.as_deref(), Some("safe"));

    let huge_version = r#"{"protocol_version":1e999999,"type":"Status","request_id":"huge"}"#;
    let error = assert_code(huge_version, RequestDecodeErrorCode::VersionInvalid);
    assert_eq!(error.request_id.as_deref(), Some("huge"));
}

fn assert_code(raw: &str, expected: RequestDecodeErrorCode) -> RequestDecodeError {
    let error = decode_request(raw).expect_err("request must be rejected");
    assert_eq!(error.code, expected);
    error
}
