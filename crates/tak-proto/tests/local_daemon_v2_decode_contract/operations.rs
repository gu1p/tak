use tak_proto::local_daemon::v2::{DecodeOutcome, Operation, PROTOCOL_VERSION, decode_request};

#[test]
fn decoder_recognizes_every_v2_run_operation() {
    assert_eq!(PROTOCOL_VERSION, 2);
    let list = v2(r#"{"protocol_version":2,"request_id":"list","operation":{"type":"ListRuns"}}"#);
    assert!(matches!(list.operation, Operation::ListRuns {}));
    assert_eq!(list.request_id, "list");

    let daemon_status = v2(
        r#"{"protocol_version":2,"request_id":"status","operation":{"type":"GetDaemonStatus"}}"#,
    );
    assert!(matches!(
        daemon_status.operation,
        Operation::GetDaemonStatus {}
    ));

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

    let outputs = v2(
        r#"{"protocol_version":2,"request_id":"outputs","operation":{"type":"GetOutputManifest","run_id":"run-4"}}"#,
    );
    assert!(matches!(
        outputs.operation,
        Operation::GetOutputManifest { run_id } if run_id == "run-4"
    ));

    let candidates = v2(
        r#"{"protocol_version":2,"request_id":"candidates","operation":{"type":"ResolveRemoteCandidates","requirements":{"pool":"build","required_tags":["builder"],"required_capabilities":["linux"],"transport":"tor"}}}"#,
    );
    assert!(matches!(
        candidates.operation,
        Operation::ResolveRemoteCandidates { requirements }
            if requirements.pool.as_deref() == Some("build")
                && requirements.transport.as_deref() == Some("tor")
    ));
}

#[test]
fn decoder_recognizes_remote_inventory_and_health_operations() {
    let add = v2(
        r#"{"protocol_version":2,"request_id":"add","operation":{"type":"AddRemote","invite":"takd:tor:secret"}}"#,
    );
    assert!(
        matches!(add.operation, Operation::AddRemote { invite } if invite == "takd:tor:secret")
    );

    let status = v2(
        r#"{"protocol_version":2,"request_id":"status","operation":{"type":"GetRemoteStatus","node_ids":["builder-a"]}}"#,
    );
    assert!(matches!(
        status.operation,
        Operation::GetRemoteStatus { node_ids } if node_ids == ["builder-a"]
    ));

    let read = v2(
        r#"{"protocol_version":2,"request_id":"read","operation":{"type":"ReadRemote","node_id":"builder-a","path":"/v2/worker/tasks?state=all&limit=10"}}"#,
    );
    assert!(matches!(
        read.operation,
        Operation::ReadRemote { node_id, path }
            if node_id == "builder-a" && path == "/v2/worker/tasks?state=all&limit=10"
    ));
}

fn v2(raw: &str) -> tak_proto::local_daemon::v2::Request {
    match decode_request(raw).expect("valid protocol") {
        DecodeOutcome::V2(request) => request,
    }
}
