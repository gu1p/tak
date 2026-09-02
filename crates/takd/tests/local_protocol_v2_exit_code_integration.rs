use tak_proto::local_daemon::v2::{Operation, Request, Response, decode_response, encode_request};
use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::raw_local_protocol::RawLocalProtocol;
use crate::support::v2_run::{scheduler::commit, submission};

#[tokio::test(flavor = "multi_thread")]
async fn local_attempt_exit_code_reaches_terminal_protocol_summary_and_event() {
    let root = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(root.path().join("state/takd.sqlite")).unwrap();
    let run_id = commit(
        &store,
        &submission("local-exit-seven", "redacted-secret"),
        "alice",
    );
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Failed {
                terminal_digest: "7".repeat(64),
                exit_code: Some(7),
            },
        )
        .unwrap();
    drop(store);

    let mut daemon = RawLocalProtocol::start_in(root.path()).await;
    let Response::RunDetails { run, .. } = send(
        &mut daemon,
        "show",
        Operation::GetRun {
            run_id: run_id.clone(),
        },
    )
    .await
    else {
        panic!("expected details")
    };
    assert_eq!(run.summary.exit_code, Some(7));
    let Response::RunEvents {
        exit_code, events, ..
    } = send(
        &mut daemon,
        "attach",
        Operation::AttachRun {
            run_id,
            after_event: 0,
        },
    )
    .await
    else {
        panic!("expected events")
    };
    assert_eq!(exit_code, Some(7));
    assert!(events.iter().any(|event| event.exit_code == Some(7)));
}

async fn send(daemon: &mut RawLocalProtocol, request_id: &str, operation: Operation) -> Response {
    let request = Request {
        request_id: request_id.into(),
        operation,
    };
    let raw = daemon.exchange(&encode_request(&request).unwrap()).await;
    assert!(!raw.contains("redacted-secret"));
    decode_response(raw.trim().as_bytes(), request_id).unwrap()
}
