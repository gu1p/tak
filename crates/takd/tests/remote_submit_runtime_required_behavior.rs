use prost::Message;
use tak_proto::{CmdStep, ErrorResponse, Step, SubmitTaskRequest, step};
use takd::{SubmitAttemptStore, handle_remote_v1_request};

use crate::support::remote_output::{empty_workspace_zip, test_context};

#[test]
fn remote_submit_without_runtime_is_rejected_as_invalid_input() {
    let context = test_context();
    let temp = tempfile::tempdir().expect("tempdir");
    let store =
        SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("submit store");

    let submit = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), "true".to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: None,
        task_label: "//apps/web:test".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: None,
    };

    let response = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");

    assert_eq!(response.status_code, 400);
    let error = ErrorResponse::decode(response.body.as_slice()).expect("decode error");
    assert_eq!(error.message, "invalid_submit_fields");
}
