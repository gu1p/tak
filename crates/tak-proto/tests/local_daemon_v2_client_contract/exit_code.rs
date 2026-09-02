use tak_proto::local_daemon::v2::{Response, RunEventKind, decode_response};

#[test]
fn failed_terminal_pages_preserve_the_exact_process_exit_code() {
    let raw = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[{"seq":1,"kind":"failed","job_id":"job-0","task_ids":["//:exec"],"node_id":"local","message":"job failed","exit_code":7}],"next_event":1,"state":"failed","terminal":true,"logs_expired":false,"exit_code":7}"#;
    let Response::RunEvents {
        events, exit_code, ..
    } = decode_response(raw, "attach").unwrap()
    else {
        panic!("expected terminal events")
    };
    assert_eq!(exit_code, Some(7));
    assert_eq!(events[0].kind, RunEventKind::Failed);
    assert_eq!(events[0].exit_code, Some(7));
}

#[test]
fn run_details_preserve_the_durable_terminal_exit_code() {
    let raw = br#"{"protocol_version":2,"type":"RunDetails","request_id":"show","run":{"summary":{"run_id":"run-1","state":"failed","created_at_ms":1,"updated_at_ms":2,"targets":["//:exec"],"total_jobs":1,"terminal_jobs":1,"exit_code":7},"jobs":[],"logs_expired":false,"outputs_expired":false}}"#;
    let Response::RunDetails { run, .. } = decode_response(raw, "show").unwrap() else {
        panic!("expected run details")
    };
    assert_eq!(run.summary.exit_code, Some(7));
}

#[test]
fn nonterminal_pages_cannot_claim_a_process_exit_code() {
    let raw = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[],"next_event":0,"state":"running","terminal":false,"logs_expired":false,"exit_code":7}"#;
    assert!(decode_response(raw, "attach").is_err());
}
