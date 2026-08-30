use tak_proto::local_daemon::v2::{
    Response, RunLifecycleState, WorkspaceDisposition, decode_response,
};

#[test]
fn correlated_submission_flow_successes_decode_without_accepting_other_shapes() {
    let submitted = br#"{"protocol_version":2,"type":"RunSubmitted","request_id":"submit","run_id":"run-1","workspace":{"status":"upload_required","next_offset":0}}"#;
    assert!(matches!(
        decode_response(submitted, "submit").unwrap(),
        Response::RunSubmitted { run_id, workspace: WorkspaceDisposition::UploadRequired { next_offset: 0 }, .. }
            if run_id == "run-1"
    ));

    let committed = br#"{"protocol_version":2,"type":"RunCommitted","request_id":"commit","run_id":"run-1","state":"queued"}"#;
    assert!(matches!(
        decode_response(committed, "commit").unwrap(),
        Response::RunCommitted { run_id, .. } if run_id == "run-1"
    ));
}

#[test]
fn attach_events_preserve_order_and_terminal_state() {
    let raw = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[{"seq":1,"kind":"queued","job_id":"job-0","task_ids":["//:check"],"node_id":null,"message":"queued"}],"next_event":1,"state":"failed","terminal":true}"#;
    let Response::RunEvents {
        events,
        next_event,
        state,
        terminal,
        ..
    } = decode_response(raw, "attach").unwrap()
    else {
        panic!("expected events")
    };
    assert_eq!(events[0].seq, 1);
    assert_eq!(next_event, 1);
    assert_eq!(state, RunLifecycleState::Failed);
    assert!(terminal);
}

#[test]
fn attachment_terminal_flag_never_claims_a_nonterminal_state_is_finished() {
    let impossible = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[],"next_event":0,"state":"running","terminal":true}"#;
    assert!(decode_response(impossible, "attach").is_err());

    let paged = br#"{"protocol_version":2,"type":"RunEvents","request_id":"attach","run_id":"run-1","events":[],"next_event":0,"state":"failed","terminal":false}"#;
    assert!(decode_response(paged, "attach").is_ok());
}
