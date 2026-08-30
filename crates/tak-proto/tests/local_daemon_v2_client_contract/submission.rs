use tak_proto::local_daemon::v2::{
    DecodeOutcome, Operation, Request, decode_request, encode_request,
};

use crate::local_daemon_v2_submission_support::{environment, sample_run};

#[test]
fn submission_upload_commit_and_output_chunk_round_trip_strictly() {
    let operations = vec![
        Operation::SubmitRun {
            idempotency_key: "idem-1".into(),
            run: Box::new(sample_run()),
            environment_values: environment(),
        },
        Operation::UploadWorkspace {
            run_id: "run-1".into(),
            workspace_fingerprint: "a".repeat(64),
            archive_size: 3,
            offset: 0,
            chunk: b"abc".to_vec(),
        },
        Operation::CommitRun {
            run_id: "run-1".into(),
        },
        Operation::GetOutputChunk {
            artifact_id: "artifact-1".into(),
            offset: 7,
            max_bytes: 1024,
        },
    ];

    for (index, operation) in operations.into_iter().enumerate() {
        let request = Request {
            request_id: format!("request-{index}"),
            operation,
        };
        let encoded = encode_request(&request).expect("encode request");
        assert!(!encoded.contains('\n'));
        assert!(matches!(
            decode_request(&encoded).expect("decode request"),
            DecodeOutcome::V2(decoded) if decoded == request
        ));
    }
}

#[test]
fn strict_submission_rejects_unknown_nested_fields() {
    let request = Request {
        request_id: "submit".into(),
        operation: Operation::SubmitRun {
            idempotency_key: "idem-1".into(),
            run: Box::new(sample_run()),
            environment_values: environment(),
        },
    };
    let mut value: serde_json::Value =
        serde_json::from_str(&encode_request(&request).unwrap()).unwrap();
    value["operation"]["run"]["workspace"]["manifest"]["entries"][0]["future"] = true.into();

    assert!(decode_request(&value.to_string()).is_err());
}
