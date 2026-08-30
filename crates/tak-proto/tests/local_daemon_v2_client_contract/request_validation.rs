use tak_proto::local_daemon::v2::{Operation, Request, RequestEncodeError, encode_request};

#[test]
fn encoder_uses_decoded_utf8_byte_bounds_for_every_identifier() {
    for valid in ["x".repeat(128), "é".repeat(64)] {
        let request = Request {
            request_id: valid.clone(),
            operation: Operation::ListRuns {},
        };
        encode_request(&request).expect("bounded request id");
        for operation in run_operations(&valid) {
            let request = Request {
                request_id: "safe".into(),
                operation,
            };
            encode_request(&request).expect("bounded run id");
        }
    }

    for invalid in invalid_identifiers() {
        let request = Request {
            request_id: invalid,
            operation: Operation::ListRuns {},
        };
        assert_eq!(
            encode_request(&request),
            Err(RequestEncodeError::RequestIdInvalid)
        );
    }

    for invalid in invalid_identifiers() {
        for operation in run_operations(&invalid) {
            let request = Request {
                request_id: "safe".into(),
                operation,
            };
            assert_eq!(
                encode_request(&request),
                Err(RequestEncodeError::RunIdInvalid)
            );
        }
    }
}

fn invalid_identifiers() -> Vec<String> {
    vec![
        String::new(),
        "line\nbreak".into(),
        "delete\u{7f}".into(),
        "x".repeat(129),
        "é".repeat(65),
    ]
}

fn run_operations(run_id: &str) -> [Operation; 4] {
    [
        Operation::GetRun {
            run_id: run_id.into(),
        },
        Operation::AttachRun {
            run_id: run_id.into(),
            after_event: 0,
        },
        Operation::CancelRun {
            run_id: run_id.into(),
        },
        Operation::GetOutputManifest {
            run_id: run_id.into(),
        },
    ]
}
