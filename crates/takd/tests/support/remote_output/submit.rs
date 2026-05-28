use prost::Message;
use tak_proto::{
    CmdStep, OutputSelector, RuntimeSpec, Step, SubmitTaskRequest, SubmitTaskResponse, step,
};
use takd::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_request};

use super::empty_workspace_zip;

pub(super) fn submit_shell_task_with_outputs_and_runtime(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    task_run_id: &str,
    command: &str,
    outputs: Vec<OutputSelector>,
    runtime: RuntimeSpec,
) -> SubmitTaskResponse {
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
        runtime: Some(runtime),
        task_label: "//apps/web:test".to_string(),
        needs: Vec::new(),
        outputs,
        session: None,
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some(format!("sh -c '{}'", command.replace('\'', "'\\''"))),
        fused_members: Vec::new(),
        execution_label: None,
        workspace_upload: None,
    };
    let submit = handle_remote_v1_request(
        context,
        store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    SubmitTaskResponse::decode(submit.body.as_slice()).expect("decode submit")
}
