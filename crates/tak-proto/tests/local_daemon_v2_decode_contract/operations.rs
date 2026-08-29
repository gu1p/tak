use tak_proto::local_daemon::v2::{DecodeOutcome, Operation, PROTOCOL_VERSION, decode_request};

#[test]
fn decoder_recognizes_every_v2_run_operation() {
    assert_eq!(PROTOCOL_VERSION, 2);
    let list = v2(r#"{"protocol_version":2,"request_id":"list","operation":{"type":"ListRuns"}}"#);
    assert!(matches!(list.operation, Operation::ListRuns {}));
    assert_eq!(list.request_id, "list");

    let show = v2(
        r#"{"protocol_version":2,"request_id":"show","operation":{"type":"GetRun","run_id":"run-1"}}"#,
    );
    assert!(matches!(show.operation, Operation::GetRun { run_id } if run_id == "run-1"));

    let attach = v2(
        r#"{"protocol_version":2,"request_id":"attach","operation":{"type":"AttachRun","run_id":"run-2","after_event":0}}"#,
    );
    assert!(matches!(
        attach.operation,
        Operation::AttachRun { run_id, after_event: 0 } if run_id == "run-2"
    ));

    let cancel = v2(
        r#"{"protocol_version":2,"request_id":"cancel","operation":{"type":"CancelRun","run_id":"run-3"}}"#,
    );
    assert!(matches!(cancel.operation, Operation::CancelRun { run_id } if run_id == "run-3"));
}

fn v2(raw: &str) -> tak_proto::local_daemon::v2::Request {
    match decode_request(raw).expect("valid protocol") {
        DecodeOutcome::V2(request) => request,
        DecodeOutcome::LegacyCandidate => panic!("expected v2 request"),
    }
}
