use tak_proto::local_daemon::v2::{
    Response, RunEventKind, RunLifecycleState, WorkspaceDisposition, decode_response,
};

#[test]
fn daemon_status_is_strictly_versioned_correlated_and_typed() {
    let raw = br#"{"protocol_version":2,"type":"DaemonStatus","request_id":"status","status":{"active_leases":0,"pending_requests":0,"limiter_count":3}}"#;
    let Response::DaemonStatus { status, .. } = decode_response(raw, "status").unwrap() else {
        panic!("expected daemon status")
    };
    assert_eq!(status.active_leases, 0);
    assert_eq!(status.pending_requests, 0);
    assert_eq!(status.limiter_count, 3);
    assert!(decode_response(raw, "other").is_err());
}

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
fn remote_candidate_snapshot_is_strictly_correlated_and_typed() {
    let raw = br#"{"protocol_version":2,"type":"RemoteCandidates","request_id":"candidates","candidates":[{"node_id":"worker-a","kind":"remote","transport":"direct","reason":"healthy protocol-v2 worker"}]}"#;
    let Response::RemoteCandidates { candidates, .. } = decode_response(raw, "candidates").unwrap()
    else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].node_id, "worker-a");
    assert!(decode_response(raw, "another-request").is_err());
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

#[test]
fn attach_log_chunks_are_binary_safe_and_shape_checked() {
    let raw = br#"{"protocol_version":2,"type":"RunEvents","request_id":"logs","run_id":"run-1","events":[{"seq":1,"kind":"stdout","job_id":"job-0","task_ids":["//:check"],"node_id":"local","message":"","chunk_base64":"AP8K"}],"next_event":1,"state":"running","terminal":false}"#;
    let Response::RunEvents { events, .. } = decode_response(raw, "logs").unwrap() else {
        panic!("expected events")
    };
    assert_eq!(events[0].kind, RunEventKind::Stdout);
    assert_eq!(events[0].chunk_base64.as_deref(), Some("AP8K"));

    for malformed in [
        br#"{"protocol_version":2,"type":"RunEvents","request_id":"logs","run_id":"run-1","events":[{"seq":1,"kind":"stdout","job_id":null,"task_ids":[],"node_id":null,"message":""}],"next_event":1,"state":"running","terminal":false}"#.as_slice(),
        br#"{"protocol_version":2,"type":"RunEvents","request_id":"logs","run_id":"run-1","events":[{"seq":1,"kind":"queued","job_id":null,"task_ids":[],"node_id":null,"message":"","chunk_base64":"AA=="}],"next_event":1,"state":"running","terminal":false}"#.as_slice(),
        br#"{"protocol_version":2,"type":"RunEvents","request_id":"logs","run_id":"run-1","events":[{"seq":1,"kind":"stderr","job_id":null,"task_ids":[],"node_id":null,"message":"","chunk_base64":"!"}],"next_event":1,"state":"running","terminal":false}"#.as_slice(),
    ] {
        assert!(decode_response(malformed, "logs").is_err());
    }
}
