#![allow(dead_code)]

use prost::Message;
use tak_proto::{
    CmdStep, ExecutionSession, GetTaskResultResponse, Step, SubmitTaskRequest, SubmitTaskResponse,
    step,
};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

use super::remote_output::{empty_workspace_zip, test_container_runtime};

pub fn submit_session_task(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    session: ExecutionSession,
) {
    let submit = SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), command.to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:test".to_string(),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: Some(session),
    };
    let response = handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    assert_eq!(response.status_code, 200);
    let ack = SubmitTaskResponse::decode(response.body.as_slice()).expect("decode submit");
    assert!(ack.accepted);
}

pub fn session(
    key: &str,
    reuse: &str,
    share_paths: Vec<tak_proto::OutputSelector>,
) -> ExecutionSession {
    ExecutionSession {
        key: key.to_string(),
        name: "rust".to_string(),
        reuse: reuse.to_string(),
        share_paths,
    }
}

pub fn assert_success(context: &RemoteNodeContext, store: &SubmitAttemptStore, task_run_id: &str) {
    let path = format!("/v1/tasks/{task_run_id}/result");
    let response =
        handle_remote_v1_request(context, store, "GET", &path, None).expect("result response");
    let result = GetTaskResultResponse::decode(response.body.as_slice()).expect("decode result");
    assert!(result.success, "task {task_run_id} failed: {result:?}");
}
