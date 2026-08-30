use tak_proto::local_daemon::v2::{
    DecodeOutcome, Operation, RequestDecodeError, RequestDecodeErrorCode, decode_request,
};

#[test]
fn strict_v2_shape_rejects_unknown_fields_and_invalid_operation_values() {
    let cases = [
        r#"{"protocol_version":2,"credential":"secret","request_id":"safe","operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"ListRuns","extra":true}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"GetRun","run_id":""}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"GetOutputManifest","run_id":"run-1","to":"checkout"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"GetOutputManifest","run_id":""}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"GetOutputChunk","run_id":"run-1","digest":"abc","offset":0}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"AttachRun","run_id":"run-1"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"AttachRun","run_id":"run-1","after_event":-1}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"Nope"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe"}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"GetRun","run_id":"one","run_id":"two"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"AttachRun","type":"CancelRun","run_id":"run-1","after_event":0}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"safe","operation":{"type":"AttachRun","run_id":"run-1","after_event":0,"after_event":1}}"#.to_string(),
    ];

    for raw in cases {
        let error = rejected(&raw);
        assert_eq!(error.request_id.as_deref(), Some("safe"));
    }
}

#[test]
fn correlation_requires_one_bounded_control_free_request_id() {
    let cases = [
        r#"{"protocol_version":2,"request_id":"first","request_id":"second","operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"","operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"line\nbreak","operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":"delete\u007fkey","operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":123,"operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"request_id":null,"operation":{"type":"ListRuns"}}"#.to_string(),
        r#"{"protocol_version":2,"operation":{"type":"ListRuns"}}"#.to_string(),
        format!(
            r#"{{"protocol_version":2,"request_id":"{}","operation":{{"type":"ListRuns"}}}}"#,
            "x".repeat(129)
        ),
    ];

    for raw in cases {
        assert_eq!(rejected(&raw).request_id, None);
    }
}

#[test]
fn identifiers_are_opaque_control_free_values_bounded_by_decoded_utf8_bytes() {
    for request_id in ["x".repeat(128), "é".repeat(64)] {
        let raw = format!(
            r#"{{"protocol_version":2,"request_id":{},"operation":{{"type":"ListRuns"}}}}"#,
            serde_json::to_string(&request_id).expect("encode id")
        );
        let DecodeOutcome::V2(request) = decode_request(&raw).expect("bounded id") else {
            panic!("expected v2 request");
        };
        assert_eq!(request.request_id, request_id);
    }

    let long_run_id = "é".repeat(65);
    let raw = format!(
        r#"{{"protocol_version":2,"request_id":"safe","operation":{{"type":"GetRun","run_id":{}}}}}"#,
        serde_json::to_string(&long_run_id).expect("encode run id")
    );
    assert_eq!(rejected(&raw).request_id.as_deref(), Some("safe"));

    let valid_run_id = "x".repeat(128);
    let raw = format!(
        r#"{{"protocol_version":2,"request_id":"safe","operation":{{"type":"GetRun","run_id":{}}}}}"#,
        serde_json::to_string(&valid_run_id).expect("encode run id")
    );
    let DecodeOutcome::V2(request) = decode_request(&raw).expect("bounded run id") else {
        panic!("expected v2 request");
    };
    assert!(matches!(request.operation, Operation::GetRun { run_id } if run_id == valid_run_id));
}

fn rejected(raw: &str) -> RequestDecodeError {
    let error = decode_request(raw).expect_err("strict request must be rejected");
    assert_eq!(error.code, RequestDecodeErrorCode::RequestInvalid);
    error
}
