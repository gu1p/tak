use tak_proto::local_daemon::v2::{
    Operation, Request, Response, WorkspaceDisposition, decode_response, encode_request,
};

use crate::support::raw_local_protocol::RawLocalProtocol;
use crate::support::v2_run::submission;

#[tokio::test(flavor = "multi_thread")]
async fn submit_disconnect_and_restart_returns_the_same_durable_run() {
    let temp = tempfile::tempdir().unwrap();
    let submitted = submission("idem-restart", "never-render-this");
    let request = Request {
        request_id: "submit-1".into(),
        operation: Operation::SubmitRun {
            idempotency_key: submitted.idempotency_key.clone(),
            run: Box::new(submitted.run.clone()),
            environment_values: submitted.environment_values.clone(),
        },
    };
    let mut first = RawLocalProtocol::start_in(temp.path()).await;
    let raw = first.exchange(&encode_request(&request).unwrap()).await;
    let Response::RunSubmitted {
        run_id, workspace, ..
    } = decode_response(raw.trim().as_bytes(), "submit-1").unwrap()
    else {
        panic!("expected submission")
    };
    assert!(matches!(
        workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 0 }
    ));
    drop(first);

    let mut restored = RawLocalProtocol::start_in(temp.path()).await;
    let raw = restored.exchange(&encode_request(&request).unwrap()).await;
    let Response::RunSubmitted {
        run_id: restored_id,
        ..
    } = decode_response(raw.trim().as_bytes(), "submit-1").unwrap()
    else {
        panic!("expected restored submission")
    };
    assert_eq!(restored_id, run_id);
    assert!(!raw.contains("never-render-this"));
}
