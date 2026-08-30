use serde_json::{Value, json};
use tak_proto::local_daemon::v2::{
    DecodeOutcome, Operation, Request, decode_request, encode_request,
};

#[test]
fn encoder_emits_one_strict_v2_frame_for_every_dark_run_operation() {
    let cases = [
        (Operation::ListRuns {}, json!({"type": "ListRuns"})),
        (
            Operation::GetRun {
                run_id: "run-1".into(),
            },
            json!({"type": "GetRun", "run_id": "run-1"}),
        ),
        (
            Operation::AttachRun {
                run_id: "run-1".into(),
                after_event: 7,
            },
            json!({"type": "AttachRun", "run_id": "run-1", "after_event": 7}),
        ),
        (
            Operation::CancelRun {
                run_id: "run-1".into(),
            },
            json!({"type": "CancelRun", "run_id": "run-1"}),
        ),
        (
            Operation::GetOutputManifest {
                run_id: "run-1".into(),
            },
            json!({"type": "GetOutputManifest", "run_id": "run-1"}),
        ),
    ];

    for (operation, expected_operation) in cases {
        let request = Request {
            request_id: "safe-request".into(),
            operation,
        };
        let encoded = encode_request(&request).expect("encode valid request");
        assert!(!encoded.contains('\n'));
        let value: Value = serde_json::from_str(&encoded).expect("decode encoded request");
        assert_eq!(value["protocol_version"], 2);
        assert_eq!(value["request_id"], "safe-request");
        assert_eq!(value["operation"], expected_operation);
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert!(matches!(
            decode_request(&encoded).expect("server accepts client frame"),
            DecodeOutcome::V2(decoded) if decoded == request
        ));
    }
}
