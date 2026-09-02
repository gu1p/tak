use tak_proto::local_daemon::v2::{Response, decode_response};

#[test]
fn run_details_explicitly_decode_log_and_output_expiration() {
    let raw = br#"{"protocol_version":2,"type":"RunDetails","request_id":"show","run":{"summary":{"run_id":"run-1","state":"succeeded","created_at_ms":1,"updated_at_ms":2,"targets":[],"total_jobs":0,"terminal_jobs":0},"jobs":[],"logs_expired":true,"outputs_expired":true}}"#;
    let Response::RunDetails { run, .. } = decode_response(raw, "show").unwrap() else {
        panic!("expected run details")
    };
    assert!(run.logs_expired);
    assert!(run.outputs_expired);
}

#[test]
fn expired_log_pages_advance_without_carrying_log_payloads() {
    let raw = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[],"next_event":7,"state":"failed","terminal":true,"logs_expired":true}"#;
    let Response::RunEvents {
        logs_expired,
        next_event,
        ..
    } = decode_response(raw, "attach").unwrap()
    else {
        panic!("expected run events")
    };
    assert!(logs_expired);
    assert_eq!(next_event, 7);

    let leaked = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[{"seq":7,"kind":"stdout","job_id":"job-0","task_ids":["//:check"],"node_id":"local","message":"","chunk_base64":"c2VjcmV0"}],"next_event":7,"state":"failed","terminal":true,"logs_expired":true}"#;
    assert!(decode_response(leaked, "attach").is_err());
}

#[test]
fn expired_output_manifests_cannot_carry_artifact_metadata() {
    let empty = br#"{"protocol_version":2,"type":"OutputManifest","request_id":"outputs","run_id":"run-1","expired":true,"artifacts":[]}"#;
    assert!(decode_response(empty, "outputs").is_ok());
    let leaked = br#"{"protocol_version":2,"type":"OutputManifest","request_id":"outputs","run_id":"run-1","expired":true,"artifacts":[{"path":"secret","entry_type":"file","executable":false,"symlink_target":null,"size":1,"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","artifact_id":"secret"}]}"#;
    assert!(decode_response(leaked, "outputs").is_err());
}
